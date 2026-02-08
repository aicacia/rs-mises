use uuid::Uuid;

use alloc::{
  string::{String, ToString},
  vec::Vec,
};

use mises_graph::{EdgeQuery, Element, Filter, NodeQuery, Query, field};

use crate::{
  CoreError, Result,
  model::{
    edge::{EdgeProps, EdgeType},
    identity::{IdentityMeta, IdentityType},
    node::{NodeMeta, NodeType},
  },
  traits::Executor,
};

#[derive(Clone)]
pub struct IdentityService<E>
where
  E: Executor,
{
  exec: E,
}

impl<E> IdentityService<E>
where
  E: Executor,
{
  pub fn new(exec: E) -> Self {
    Self { exec }
  }

  pub async fn get_identity_type(&self, id: Uuid) -> Result<IdentityType> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match &node.metadata {
      NodeMeta::Identity(identity_meta) => Ok(identity_meta.identity_type()),
      _ => Err(CoreError::InvalidInput(crate::error::InvalidInput::Other(
        "node is not an identity".into(),
      ))),
    }
  }

  pub async fn get_node_by_id_and_identity_type(
    &self,
    id: Uuid,
    expected: IdentityType,
  ) -> Result<E::Node> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match &node.metadata {
      NodeMeta::Identity(identity_meta) => {
        if identity_meta.identity_type() == expected {
          Ok(node)
        } else {
          Err(CoreError::InvalidInput(crate::error::InvalidInput::Other(
            "identity type mismatch".into(),
          )))
        }
      }
      _ => Err(CoreError::InvalidInput(crate::error::InvalidInput::Other(
        "node is not an identity".into(),
      ))),
    }
  }

  /// Find the owner identity of the given node id
  pub async fn find_owner(&self, id: Uuid) -> Result<Option<E::Node>> {
    let query = Query::edges(
      EdgeQuery::incoming(EdgeType::Owns.as_str())
        .from(NodeQuery::new(NodeType::Identity.as_str()))
        .to(NodeQuery::any().filter(field("id").eq(id.to_string()))),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Edge(edge) = el
        && edge.r#type == EdgeType::Owns.as_str()
        && let Some(node) = self.exec.get_node_by_id(edge.from_id).await?
      {
        return Ok(Some(node));
      }
    }

    Ok(None)
  }

  /// Create a new user identity with the given name. Returns the created user's id.
  pub async fn create_user(&self, name: String) -> Result<E::Node> {
    let user_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::User { name, local: true }),
      )
      .await?;

    Ok(user_node)
  }

  /// Create a member `OWNS` edge.
  pub async fn set_owner(&self, owner_id: Uuid, target_id: Uuid) -> Result<()> {
    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        owner_id,
        target_id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    Ok(())
  }

  /// Convenience: start a transaction, set the owner, commit. Only available
  /// when the underlying executor is a full `Repository`.
  pub async fn find_root_devices(&self, root_group_id: Uuid) -> Result<Vec<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::Device.as_str())
          .into(),
        field("metadata.root").eq(root_group_id.to_string()).into(),
      ])),
    );

    let elements = self.exec.query(query).await?;
    let mut devices = Vec::new();

    for el in elements {
      if let Element::Node(node) = el {
        devices.push(node);
      }
    }

    Ok(devices)
  }

  /// Create a device node. Optionally set `root` in metadata (does not create any edges).
  pub async fn create_device(&self, device_name: String, root: Uuid) -> Result<E::Node> {
    let device_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::Device {
          name: device_name,
          local: true,
          root: Some(root),
        }),
      )
      .await?;

    Ok(device_node)
  }

  /// Add a `MEMBER_OF` edge attaching `member_id` to `group_id`.
  pub async fn add_member_of(
    &self,
    member_id: Uuid,
    group_id: Uuid,
    now: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<()> {
    self
      .exec
      .create_edge(
        EdgeType::MemberOf.as_str().to_string(),
        member_id,
        group_id,
        EdgeProps::MemberOf {
          since: now,
          until: now,
        },
      )
      .await?;

    Ok(())
  }
}
