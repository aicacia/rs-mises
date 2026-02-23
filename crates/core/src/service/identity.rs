use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec::Vec,
};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use uuid::Uuid;

use mises_graph::{EdgeQuery, Element, Filter, NodeQuery, Query, field};
use mises_key::Key;

use crate::{
  CoreError, InvalidInput, Result,
  model::{
    edge::{EdgeProps, EdgeType},
    identity::{IdentityMeta, IdentityType},
    keys::{KeyMaterial, KeyMeta},
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
  device_id: String,
}

impl<E> IdentityService<E>
where
  E: Executor,
{
  /// Create a new `IdentityService` with the given executor and device ID.
  pub fn new(exec: E, device_id: String) -> Self {
    Self { exec, device_id }
  }

  /// Get a node by ID and verify it matches the expected identity type.
  ///
  /// # Arguments
  ///
  /// * `id` - The UUID of the node to retrieve
  /// * `expected` - The `IdentityType` that the node must be
  ///
  /// # Returns
  ///
  /// The node if found and the identity type matches
  ///
  /// # Errors
  ///
  /// Returns an error if the node is not found, is not an identity, or the type mismatches.
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

  /// Find the owner of an identity with optional type filtering.
  pub async fn find_owner(
    &self,
    id: Uuid,
    owner_type: Option<IdentityType>,
  ) -> Result<Option<E::Node>> {
    let owner_query = if let Some(itype) = owner_type {
      NodeQuery::new(NodeType::Identity.as_str()).filter(field("metadata.type").eq(itype.as_str()))
    } else {
      NodeQuery::new(NodeType::Identity.as_str())
    };

    let query = Query::nodes(
      owner_query.include(
        EdgeQuery::outgoing(EdgeType::Owns.as_str())
          .to(NodeQuery::any().filter(field("id").eq(id.to_string()))),
      ),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(Some(node));
      }
    }

    Ok(None)
  }

  async fn create_key_for_identity(
    &self,
    identity_id: Uuid,
    identity_type: IdentityType,
  ) -> Result<E::Node> {
    let master_key = self.get_master_key().await?;

    let identity_index = Self::uuid_to_u32(identity_id);
    let type_index = identity_type.as_u32();
    let child_path = format!("m/44'/0/{}/{}", type_index, identity_index);

    log::debug!(
      "create_key_for_identity: deriving child key for {} with path: {}",
      identity_type.as_str(),
      child_path
    );

    let child_key = master_key
      .child_from_derivation_path(&child_path)
      .map_err(|e| CoreError::other(InvalidInput::Other(format!("key derivation error: {}", e))))?;

    log::debug!(
      "create_key_for_identity: derived child, derivation_path={}",
      child_key.derivation_path()
    );

    let kp = child_key.ed25519_keypair()?;
    let public_key = BASE64_URL_SAFE.encode(kp.public.as_bytes());

    let master_key = self.get_master_key().await?;
    let master_seed = master_key
      .seed_bytes()
      .ok_or_else(|| CoreError::other(InvalidInput::Other("master seed not available".into())))?;
    let master_seed_b64 = BASE64_URL_SAFE.encode(&master_seed);

    let key_node = self
      .exec
      .create_node(
        NodeType::Key.as_str().to_string(),
        NodeMeta::Key(KeyMeta {
          public_key,
          private_key: Some(master_seed_b64),
          derivation_path: child_key.derivation_path(),
          key_material: KeyMaterial::Seed,
        }),
      )
      .await?;

    log::debug!(
      "create_key_for_identity: stored terminal key node {} (no re-derivation needed)",
      key_node.id
    );

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
          key_material: KeyMaterial::Seed,
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

  fn uuid_to_u32(id: Uuid) -> u32 {
    let bytes = id.as_bytes();
    let a = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let b = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let c = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let d = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    (a ^ b ^ c ^ d) & 0x7fffffff
  }

  pub async fn create_user(
    &self,
    name: String,
    encrypted_password: String,
    group_id: Option<Uuid>,
  ) -> Result<(E::Node, E::Node)> {
    let user_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(Box::new(IdentityMeta::User {
          name,
          encrypted_password,
          force_password_reset: None,
        })),
      )
      .await?;

    let key_node = self
      .create_key_for_identity(user_node.id, IdentityType::User)
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

    let group_id = match group_id {
      Some(id) => id,
      None => self.get_master_group().await?.id,
    };

    self.add_member_of(user_node.id, group_id, None).await?;

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

  pub async fn get_identity_key(&self, identity_id: Uuid) -> Result<KeyMeta> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Key.as_str()).include(
        EdgeQuery::incoming(EdgeType::Owns.as_str())
          .from(NodeQuery::any().filter(field("id").eq(identity_id.to_string()))),
      ),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(key_node) = el
        && let NodeMeta::Key(key_meta) = key_node.metadata
      {
        return Ok(key_meta);
      }
    }

    Err(CoreError::NotFound)
  }

  pub async fn get_key_by_id(&self, key_id: Uuid) -> Result<KeyMeta> {
    let node = self
      .exec
      .get_node_by_id(key_id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match node.metadata {
      NodeMeta::Key(key_meta) => Ok(key_meta),
      _ => Err(CoreError::InvalidInput(InvalidInput::Other(
        "node is not a key".into(),
      ))),
    }
  }

  pub async fn get_identity_key_node(&self, identity_id: Uuid) -> Result<E::Node> {
    let edge_query = Query::edges(
      EdgeQuery::outgoing(EdgeType::Owns.as_str())
        .from(NodeQuery::any().filter(field("id").eq(identity_id.to_string()))),
    );

    let edge_elements = self.exec.query(edge_query).await?;

    let mut key_ids: Vec<_> = Vec::new();
    for el in edge_elements {
      if let Element::Edge(edge) = el {
        log::debug!(
          "get_identity_key_node: found Owns edge from {} to {}",
          edge.from_id,
          edge.to_id
        );
        key_ids.push(edge.to_id);
      }
    }

    for key_id in key_ids {
      if let Some(key_node) = self.exec.get_node_by_id(key_id).await?
        && let NodeMeta::Key(key_meta) = &key_node.metadata
      {
        log::debug!(
          "get_identity_key_node found key for identity {}: derivation_path={}, public_key={}",
          identity_id,
          key_meta.derivation_path,
          key_meta.public_key
        );
        return Ok(key_node);
      }
    }

    Err(CoreError::NotFound)
  }

  pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<E::Node> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::User.as_str())
          .into(),
        field("metadata.name").eq(username.to_string()).into(),
      ])),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Identity(identity_meta) = &node.metadata
        && let IdentityMeta::User {
          name,
          encrypted_password,
          ..
        } = identity_meta.as_ref()
        && name == username
      {
        let is_valid = crate::service::password::verify_password(password, encrypted_password)?;

        if is_valid {
          return Ok(node);
        }
      }
    }

    Err(CoreError::NotFound)
  }

  pub async fn verify_ownership(&self, owner_id: Uuid, owned_id: Uuid) -> Result<bool> {
    let query = Query::edges(
      EdgeQuery::incoming(EdgeType::Owns.as_str())
        .from(NodeQuery::any().filter(field("id").eq(owner_id.to_string())))
        .to(NodeQuery::any().filter(field("id").eq(owned_id.to_string()))),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Edge(edge) = el
        && edge.r#type == EdgeType::Owns.as_str()
        && edge.from_id == owner_id
        && edge.to_id == owned_id
      {
        return Ok(true);
      }
    }

    Ok(false)
  }

  pub async fn find_owned_identities(
    &self,
    owner_id: Uuid,
    identity_type: Option<IdentityType>,
  ) -> Result<Vec<E::Node>> {
    let to_query = if let Some(itype) = identity_type {
      NodeQuery::new(NodeType::Identity.as_str()).filter(field("metadata.type").eq(itype.as_str()))
    } else {
      NodeQuery::new(NodeType::Identity.as_str())
    };

    let query = Query::edges(
      EdgeQuery::incoming(EdgeType::Owns.as_str())
        .from(NodeQuery::any().filter(field("id").eq(owner_id.to_string())))
        .to(to_query),
    );

    let elements = self.exec.query(query).await?;
    let mut identities = Vec::new();

    for el in elements {
      if let Element::Node(node) = el {
        identities.push(node);
      }
    }

    Ok(identities)
  }

  pub async fn find_service_by_name(&self, name: &str) -> Result<Option<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::Service.as_str())
          .into(),
        field("metadata.name").eq(name.to_string()).into(),
      ])),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(Some(node));
      }
    }

    Ok(None)
  }

  pub async fn find_application_by_name(&self, name: &str) -> Result<Option<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::Application.as_str())
          .into(),
        field("metadata.oidc.client_name")
          .eq(name.to_string())
          .into(),
      ])),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(Some(node));
      }
    }

    Ok(None)
  }

  pub async fn get_master_group(&self) -> Result<E::Node> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Key.as_str()).filter(field("metadata.derivation_path").eq("m/44'")),
    );
    let elements = self.exec.query(query).await?;

    let master_key_node_id = elements
      .iter()
      .find_map(|el| {
        if let Element::Node(node) = el {
          Some(node.id)
        } else {
          None
        }
      })
      .ok_or(CoreError::other(InvalidInput::Other(
        "master key not found".into(),
      )))?;

    let owner_query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str())
        .filter(field("metadata.type").eq(IdentityType::Group.as_str()))
        .include(
          EdgeQuery::outgoing(EdgeType::Owns.as_str())
            .to(NodeQuery::any().filter(field("id").eq(master_key_node_id.to_string()))),
        ),
    );

    let elements = self.exec.query(owner_query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(node);
      }
    }

    Err(CoreError::other(InvalidInput::Other(
      "master group not found".into(),
    )))
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

  pub async fn create_device(
    &self,
    name: String,
    root: Option<Uuid>,
    device_id: Option<String>,
    group_id: Option<Uuid>,
  ) -> Result<(E::Node, E::Node)> {
    let device_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(Box::new(IdentityMeta::Device {
          name,
          root,
          device_id,
        })),
      )
      .await?;

    let key_node = self
      .create_key_for_identity(device_node.id, IdentityType::Device)
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

    let group_id = match group_id {
      Some(id) => id,
      None => self.get_master_group().await?.id,
    };
    self.add_member_of(device_node.id, group_id, None).await?;

    Ok((device_node, key_node))
  }

  pub async fn find_this_device(&self, root_group_id: Uuid) -> Result<Option<E::Node>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::Device.as_str())
          .into(),
        field("metadata.root").eq(root_group_id.to_string()).into(),
        field("metadata.device_id")
          .eq(self.device_id.clone())
          .into(),
      ])),
    );

    let elements = self.exec.query(query).await?;

    for el in elements {
      if let Element::Node(node) = el {
        return Ok(Some(node));
      }
    }

    Ok(None)
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
    let group_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(Box::new(IdentityMeta::Group { name })),
      )
      .await?;

    let key_node = self
      .create_key_for_identity(group_node.id, IdentityType::Group)
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

  pub async fn create_application(
    &self,
    group_id: Option<Uuid>,
    oidc: crate::model::oidc::OidcClientMeta,
  ) -> Result<(E::Node, E::Node)> {
    let app_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(Box::new(IdentityMeta::Application {
          oidc: Box::new(oidc),
        })),
      )
      .await?;

    let key_node = self
      .create_key_for_identity(app_node.id, IdentityType::Application)
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

    let group_id = match group_id {
      Some(id) => id,
      None => self.get_master_group().await?.id,
    };

    self.add_member_of(app_node.id, group_id, None).await?;

    Ok((app_node, key_node))
  }

  pub async fn create_service(
    &self,
    name: String,
    group_id: Option<Uuid>,
  ) -> Result<(E::Node, E::Node)> {
    let service_node = self
      .exec
      .create_node(
        NodeType::Identity.as_str().to_string(),
        NodeMeta::Identity(Box::new(IdentityMeta::Service { name })),
      )
      .await?;

    let key_node = self
      .create_key_for_identity(service_node.id, IdentityType::Service)
      .await?;

    self
      .exec
      .create_edge(
        EdgeType::Owns.as_str().to_string(),
        service_node.id,
        key_node.id,
        EdgeProps::Owns {
          since: None,
          until: None,
        },
      )
      .await?;

    let group_id = match group_id {
      Some(id) => id,
      None => self.get_master_group().await?.id,
    };

    self.add_member_of(service_node.id, group_id, None).await?;

    Ok((service_node, key_node))
  }
}
