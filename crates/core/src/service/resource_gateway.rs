use alloc::{
  collections::BTreeSet,
  string::{String, ToString},
  vec::Vec,
};

use uuid::Uuid;

use mises_graph::{EdgeQuery, Element, NodeQuery, Query, field};

use crate::{
  CoreError, InvalidInput, Result,
  model::{
    edge::EdgeType,
    node::{NodeMeta, NodeType},
  },
  traits::Repository,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleResource {
  pub resource_id: Uuid,
  pub resource_type: String,
  pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAccessOperation {
  Read,
  Write,
}

#[derive(Clone)]
pub struct ResourceGatewayService<E>
where
  E: Repository,
{
  exec: E,
}

impl<E> ResourceGatewayService<E>
where
  E: Repository,
{
  pub fn new(exec: E) -> Self {
    Self { exec }
  }

  pub async fn list_accessible_resources(
    &self,
    identity_id: Uuid,
    resource_type: Option<&str>,
  ) -> Result<Vec<AccessibleResource>> {
    self.ensure_identity_exists(identity_id).await?;

    let mut seen = BTreeSet::new();
    let query = Query::edges(
      EdgeQuery::outgoing(EdgeType::Owns.as_str())
        .from(
          NodeQuery::new(NodeType::Identity.as_str())
            .filter(field("id").eq(identity_id.to_string())),
        )
        .to(NodeQuery::new(NodeType::Resource.as_str())),
    );

    let elements = self.exec.query(query).await?;
    let mut resources = Vec::new();

    for element in elements {
      if let Element::Edge(edge) = element {
        if !seen.insert(edge.to_id) {
          continue;
        }

        let Some(resource_node) = self.exec.get_node_by_id(edge.to_id).await? else {
          continue;
        };

        let NodeMeta::Resource(resource) = resource_node.metadata else {
          continue;
        };

        if let Some(filter) = resource_type
          && resource.r#type != filter
        {
          continue;
        }

        resources.push(AccessibleResource {
          resource_id: edge.to_id,
          resource_type: resource.r#type,
          permissions: resource.permissions,
        });
      }
    }

    resources.sort_by_key(|resource| resource.resource_id);

    Ok(resources)
  }

  pub async fn get_accessible_resource(
    &self,
    identity_id: Uuid,
    resource_id: Uuid,
  ) -> Result<Option<AccessibleResource>> {
    let resources = self
      .list_accessible_resources(identity_id, None)
      .await?
      .into_iter()
      .find(|resource| resource.resource_id == resource_id);

    Ok(resources)
  }

  pub async fn check_file_access(
    &self,
    identity_id: Uuid,
    resource_id: Uuid,
    operation: FileAccessOperation,
  ) -> Result<()> {
    let required_action = match operation {
      FileAccessOperation::Read => "read",
      FileAccessOperation::Write => "write",
    };

    let resource = self
      .get_accessible_resource(identity_id, resource_id)
      .await?
      .ok_or(CoreError::NotFound)?;

    let has_permission = resource.permissions.iter().any(|permission| {
      let lower = permission.to_lowercase();
      lower == required_action
        || lower == "readwrite"
        || lower == "all"
        || (required_action == "read" && lower == "readonly")
    });

    if has_permission {
      Ok(())
    } else {
      Err(CoreError::Forbidden)
    }
  }

  async fn ensure_identity_exists(&self, id: Uuid) -> Result<()> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match node.metadata {
      NodeMeta::Identity(_) => Ok(()),
      _ => Err(CoreError::InvalidInput(InvalidInput::Other(
        "expected identity node".to_string(),
      ))),
    }
  }
}
