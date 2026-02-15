use uuid::Uuid;

use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_graph::{EdgeQuery, Element, Filter, NodeQuery, Query, field};
use mises_key::Key;

use crate::{
  CoreError, InvalidInput, Result,
  model::{
    edge::{EdgeProps, EdgeType},
    identity::{IdentityMeta, IdentityType},
    keys::KeyMeta,
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

  async fn create_key_for_identity(&self) -> Result<E::Node> {
    let master_key = self.get_master_key().await?;

    let identity_counter = self.get_next_identity_counter().await?;
    let child_path = format!("m/44'/{}'/0'", identity_counter);

    let child_key = master_key
      .child_from_derivation_path(&child_path)
      .map_err(|e| CoreError::other(InvalidInput::Other(format!("key derivation error: {}", e))))?;

    let kp = child_key.ed25519_keypair()?;
    let public_key = BASE64_URL_SAFE.encode(kp.public.as_bytes());

    let seed_bytes = master_key
      .seed_bytes()
      .ok_or(CoreError::other(InvalidInput::Other(
        "master key missing seed".into(),
      )))?;
    let private_key_b64 = BASE64_URL_SAFE.encode(seed_bytes.as_slice());

    let key_node = self
      .exec
      .create_node(
        NodeType::Key.as_str().to_string(),
        NodeMeta::Key(KeyMeta {
          public_key,
          private_key: Some(private_key_b64),
          derivation_path: child_key.derivation_path(),
        }),
      )
      .await?;

    Ok(key_node)
  }

  async fn get_master_key(&self) -> Result<Key> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Key.as_str()).filter(field("metadata.derivation_path").eq("m/44'")),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Key(KeyMeta {
          private_key: Some(b64),
          ..
        }) = &node.metadata
      {
        let bytes = BASE64_URL_SAFE.decode(b64.as_bytes()).map_err(|e| {
          CoreError::other(InvalidInput::Other(format!("base64 decode error: {}", e)))
        })?;
        let key = Key::from_master_seed_bytes(bytes).map_err(|e| {
          CoreError::other(InvalidInput::Other(format!("invalid key bytes: {}", e)))
        })?;
        return Ok(key);
      }
    }

    Err(CoreError::other(InvalidInput::Other(
      "master key not found".into(),
    )))
  }

  async fn get_next_identity_counter(&self) -> Result<u32> {
    let query = Query::nodes(NodeQuery::new(NodeType::Key.as_str()));

    let elements = self.exec.query(query).await?;
    let mut max_counter = 0u32;

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Key(KeyMeta {
          derivation_path, ..
        }) = &node.metadata
        && let Some(counter_str) = derivation_path
          .strip_prefix("m/44'/")
          .and_then(|s| s.split('/').next())
        && let Ok(counter) = counter_str.parse::<u32>()
      {
        max_counter = max_counter.max(counter);
      }
    }

    Ok(max_counter.saturating_add(1))
  }

  pub async fn create_user(
    &self,
    name: String,
    encrypted_password: String,
  ) -> Result<(E::Node, E::Node)> {
    let key_node = self.create_key_for_identity().await?;

    let user_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::User {
          name,
          encrypted_password,
          force_password_reset: None,
        }),
      )
      .await?;

    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        user_node.id,
        key_node.id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    Ok((user_node, key_node))
  }

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

  pub async fn find_any_application(&self) -> Result<Option<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str())
        .filter(field("metadata.type").eq(IdentityType::Application.as_str())),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(Some(node));
      }
    }

    Ok(None)
  }

  pub async fn list_applications(&self) -> Result<Vec<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str())
        .filter(field("metadata.type").eq(IdentityType::Application.as_str())),
    );

    let elements = self.exec.query(query).await?;

    let mut applications = Vec::new();

    for el in elements {
      if let Element::Node(node) = el {
        applications.push(node);
      }
    }

    Ok(applications)
  }

  pub async fn create_device(&self, device_name: String, root: Uuid) -> Result<(E::Node, E::Node)> {
    let key_node = self.create_key_for_identity().await?;

    let device_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::Device {
          name: device_name,
          root: Some(root),
        }),
      )
      .await?;

    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        device_node.id,
        key_node.id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    Ok((device_node, key_node))
  }

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

  pub async fn create_group(&self, name: String) -> Result<(E::Node, E::Node)> {
    let key_node = self.create_key_for_identity().await?;

    let group_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::Group { name }),
      )
      .await?;

    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        group_node.id,
        key_node.id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    Ok((group_node, key_node))
  }

  pub async fn create_application(&self, name: String) -> Result<(E::Node, E::Node)> {
    let key_node = self.create_key_for_identity().await?;

    let app_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(IdentityMeta::Application { name, oidc: None }),
      )
      .await?;

    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        app_node.id,
        key_node.id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    Ok((app_node, key_node))
  }
}
