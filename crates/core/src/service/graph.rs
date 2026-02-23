use alloc::{
  borrow::ToOwned,
  format,
  string::{String, ToString},
  vec::Vec,
};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use mises_graph::{
  Element, Executor as MisesGraphExecutor, Filter, NodeQuery, Query, Transaction, field,
};
use mises_key::Key;

use crate::{
  CoreError, InvalidInput, Result,
  model::{
    identity::IdentityType,
    keys::{KeyMaterial, KeyMeta},
    node::{NodeMeta, NodeType},
  },
  service::{identity::IdentityService, password::hash_password},
  traits::{Executor, Repository},
};

#[derive(Clone)]
pub struct BootstrapOptions {
  /// Device identifier for the bootstrapped system
  pub device_id: String,
  /// Optional name for the device
  pub device_name: Option<String>,
  /// Optional name for the root group
  pub root_group_name: Option<String>,
  /// Optional name for the owner
  pub owner_name: Option<String>,
  /// Optional timestamp for bootstrap operations (defaults to current time)
  pub now: Option<DateTime<Utc>>,
}

impl BootstrapOptions {
  /// Create a builder for constructing `BootstrapOptions`.
  pub fn builder(device_id: impl Into<String>) -> BootstrapOptionsBuilder {
    BootstrapOptionsBuilder {
      device_id: device_id.into(),
      device_name: None,
      root_group_name: None,
      owner_name: None,
      now: None,
    }
  }
}

/// Builder for constructing `BootstrapOptions`.
#[derive(Clone, Default)]
pub struct BootstrapOptionsBuilder {
  device_id: String,
  device_name: Option<String>,
  root_group_name: Option<String>,
  owner_name: Option<String>,
  now: Option<DateTime<Utc>>,
}

impl BootstrapOptionsBuilder {
  /// Create a new builder with required device ID.
  pub fn new(device_id: impl Into<String>) -> Self {
    Self {
      device_id: device_id.into(),
      device_name: None,
      root_group_name: None,
      owner_name: None,
      now: None,
    }
  }

  /// Set the device name.
  pub fn device_name(mut self, name: impl Into<String>) -> Self {
    self.device_name = Some(name.into());
    self
  }

  /// Set the root group name.
  pub fn root_group_name(mut self, name: impl Into<String>) -> Self {
    self.root_group_name = Some(name.into());
    self
  }

  /// Set the owner name.
  pub fn owner_name(mut self, name: impl Into<String>) -> Self {
    self.owner_name = Some(name.into());
    self
  }

  /// Set the bootstrap timestamp.
  pub fn now(mut self, now: DateTime<Utc>) -> Self {
    self.now = Some(now);
    self
  }

  /// Build the `BootstrapOptions`.
  pub fn build(self) -> BootstrapOptions {
    BootstrapOptions {
      device_id: self.device_id,
      device_name: self.device_name,
      root_group_name: self.root_group_name,
      owner_name: self.owner_name,
      now: self.now,
    }
  }
}

/// Result of a bootstrap operation containing identifiers for created entities.
pub struct BootstrapResult {
  /// UUID of the root group
  pub root_group_id: Uuid,
  /// Base64-encoded public key of the master key
  pub master_key_public_key: String,
  /// Whether a new master key was created during bootstrap
  pub master_key_created: bool,
  /// UUID of the owner user
  pub owner_user_id: Uuid,
  /// UUID of the device
  pub device_id: Uuid,
  /// UUID of the service
  pub service_id: Uuid,
}

/// Service for managing graph operations including bootstrap and data initialization.
#[derive(Clone)]
pub struct GraphService<E>
where
  E: Executor,
{
  exec: E,
}

impl<E> GraphService<E>
where
  E: Executor,
{
  /// Create a new `GraphService` with the given executor.
  pub fn new(exec: E) -> Self {
    Self { exec }
  }

  async fn get_or_create_master_key(&self) -> Result<(Key, Vec<u8>, bool)> {
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
        let key = Key::from_master_seed_bytes(bytes.clone()).map_err(|e| {
          CoreError::other(InvalidInput::Other(format!("invalid key bytes: {}", e)))
        })?;
        return Ok((key, bytes, false));
      }
    }

    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
      .map_err(|e| CoreError::other(InvalidInput::Other(format!("getrandom error: {}", e))))?;

    let key = Key::from_entropy(&entropy).map_err(CoreError::from)?;

    Ok((key, entropy.to_vec(), true))
  }

  pub async fn bootstrap(&self, options: BootstrapOptions) -> Result<BootstrapResult>
  where
    E: Repository + Clone,
  {
    let identity = IdentityService::new(self.exec.clone(), options.device_id.clone());

    let (master_key, seed_bytes, master_key_created): (Key, Vec<u8>, bool) =
      self.get_or_create_master_key().await?;
    let kp = master_key.ed25519_keypair()?;
    let master_key_public_key = BASE64_URL_SAFE.encode(kp.public.as_bytes());

    if master_key_created {
      log::debug!("A new master key was created");

      let tx = self.exec.transaction().await?;

      let _key_node = tx
        .create_node(
          NodeType::Key.as_str().to_string(),
          NodeMeta::Key(KeyMeta {
            public_key: master_key_public_key.clone(),
            private_key: Some(BASE64_URL_SAFE.encode(seed_bytes.as_slice())),
            derivation_path: master_key.derivation_path(),
            key_material: KeyMaterial::Seed,
          }),
        )
        .await?;

      tx.commit().await?;
    }

    let master_group_query = Query::nodes(
      NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
        field("metadata.type")
          .eq(IdentityType::Group.as_str())
          .into(),
        field("metadata.name").eq("master".to_string()).into(),
      ])),
    );

    let master_group_elements = self.exec.query(master_group_query).await?;
    let root_group_id = if let Some(Element::Node(node)) = master_group_elements.first() {
      node.id
    } else {
      log::debug!("Master group not found, creating new master group");
      let (group_node, _key_node) = identity.create_group("master".to_string()).await?;
      log::debug!("Created master group with id {}", group_node.id);
      group_node.id
    };

    let owner_user_id = match identity
      .find_owner(root_group_id, Some(IdentityType::User))
      .await?
    {
      Some(owner_node) => owner_node.id,
      None => {
        let (user_node, _key_node) = identity
          .create_user(
            options
              .owner_name
              .clone()
              .unwrap_or_else(|| "admin".to_owned()),
            hash_password("admin")?,
            Some(root_group_id),
          )
          .await?;

        log::debug!(
          "bootstrap: created owner user {} for group {}",
          user_node.id,
          root_group_id
        );

        log::debug!(
          "Created owner user with id {} for master group {}",
          user_node.id,
          root_group_id
        );

        user_node.id
      }
    };

    let device_id_str = options.device_id.clone();
    let device_id = match identity.find_this_device(root_group_id).await? {
      Some(did_node) => did_node.id,
      None => {
        let (device_node, _key_node) = identity
          .create_device(
            options
              .device_name
              .clone()
              .unwrap_or_else(|| options.device_id.clone()),
            Some(root_group_id),
            Some(device_id_str),
            Some(root_group_id),
          )
          .await?;

        log::debug!(
          "Created device with id {} belonging to master group {} and hardware device id {}",
          device_node.id,
          root_group_id,
          options.device_id
        );
        device_node.id
      }
    };

    let service_id = match identity.find_service_by_name("mises").await? {
      Some(service_node) => service_node.id,
      None => {
        let (service_node, _key_node) = identity.create_service("mises".to_owned(), None).await?;

        log::debug!("Created service identity with id {}", service_node.id);

        service_node.id
      }
    };

    Ok(BootstrapResult {
      root_group_id,
      master_key_public_key,
      master_key_created,
      owner_user_id,
      device_id,
      service_id,
    })
  }

  pub async fn list_keys(&self) -> Result<Vec<(uuid::Uuid, KeyMeta)>> {
    let query = Query::nodes(NodeQuery::new(NodeType::Key.as_str()));
    let elements = self.exec.query(query).await?;

    let mut out: Vec<(uuid::Uuid, KeyMeta)> = Vec::new();

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Key(km) = node.metadata
      {
        out.push((node.id, km));
      }
    }

    Ok(out)
  }
}
