use crate::CoreError;
use crate::service::identity::IdentityService;
use alloc::format;
use alloc::{
  borrow::ToOwned,
  string::{String, ToString},
  vec::Vec,
};
use base64::{Engine, prelude::BASE64_URL_SAFE};
use chrono::{DateTime, Utc};
use mises_graph::Executor as MisesGraphExecutor;
use mises_graph::{EdgeQuery, Element, NodeQuery, Query, Transaction, field};

use mises_key::Key;
use uuid::Uuid;

use crate::{
  model::{
    edge::{EdgeProps, EdgeType},
    identity::IdentityMeta,
    keys::KeyMeta,
    node::NodeMeta,
  },
  {
    Result,
    model::{identity::IdentityType, node::NodeType},
    traits::{Executor, Repository},
  },
};

use crate::InvalidInput;

#[derive(Clone)]
pub struct BootstrapOptions {
  pub root_group_name: Option<String>,
  pub owner_name: Option<String>,
  pub device_name: Option<String>,
  pub now: Option<DateTime<Utc>>,
  pub test_seed: Option<Vec<u8>>,
}

impl BootstrapOptions {
  pub fn builder() -> BootstrapOptionsBuilder {
    BootstrapOptionsBuilder {
      root_group_name: None,
      owner_name: None,
      device_name: None,
      now: None,
      test_seed: None,
    }
  }
}

#[derive(Clone, Default)]
pub struct BootstrapOptionsBuilder {
  root_group_name: Option<String>,
  owner_name: Option<String>,
  device_name: Option<String>,
  now: Option<DateTime<Utc>>,
  test_seed: Option<Vec<u8>>,
}

impl BootstrapOptionsBuilder {
  pub fn new() -> Self {
    Self {
      root_group_name: None,
      owner_name: None,
      device_name: None,
      now: None,
      test_seed: None,
    }
  }

  pub fn root_group_name(mut self, name: impl Into<String>) -> Self {
    self.root_group_name = Some(name.into());
    self
  }

  pub fn owner_name(mut self, name: impl Into<String>) -> Self {
    self.owner_name = Some(name.into());
    self
  }

  pub fn device_name(mut self, name: impl Into<String>) -> Self {
    self.device_name = Some(name.into());
    self
  }

  pub fn now(mut self, now: DateTime<Utc>) -> Self {
    self.now = Some(now);
    self
  }

  pub fn test_seed(mut self, seed: Vec<u8>) -> Self {
    self.test_seed = Some(seed);
    self
  }

  pub fn build(self) -> Result<BootstrapOptions> {
    Ok(BootstrapOptions {
      root_group_name: self.root_group_name,
      owner_name: self.owner_name,
      device_name: self.device_name,
      now: self.now,
      test_seed: self.test_seed,
    })
  }
}

pub struct BootstrapResult {
  pub root_group: Uuid,
  pub master_key_public_key: String,
  pub master_key_created: bool,
  pub owner_user: Uuid,
  pub device: Uuid,
}

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
  pub fn new(exec: E) -> Self {
    Self { exec }
  }

  async fn get_or_create_master_key(
    &self,
    options: &BootstrapOptions,
  ) -> Result<(Key, Vec<u8>, bool)> {
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
        return Ok((Key::from(bytes.clone()), bytes, false));
      }
    }

    let mut entropy = [0u8; 32];
    if let Some(seed) = options.test_seed.as_ref() {
      if seed.len() != 32 {
        return Err(CoreError::InvalidInput(InvalidInput::Other(
          "test_seed must be 32 bytes".to_string(),
        )));
      }
      entropy.copy_from_slice(&seed[..32]);
    } else {
      getrandom::getrandom(&mut entropy)
        .map_err(|e| CoreError::other(InvalidInput::Other(format!("getrandom error: {}", e))))?;
    }
    let key = Key::from_entropy(&entropy).map_err(CoreError::from)?;

    Ok((key, entropy.to_vec(), true))
  }

  pub async fn bootstrap(&self, options: BootstrapOptions) -> Result<BootstrapResult>
  where
    E: Repository + Clone,
  {
    let group_node = {
      let query = Query::nodes(
        NodeQuery::new(NodeType::Key.as_str())
          .filter(field("metadata.derivation_path").eq("m/44'"))
          .include(
            EdgeQuery::incoming(EdgeType::Owns.as_str()).from(
              NodeQuery::new(NodeType::Identity.as_str())
                .filter(field("metadata.type").eq(IdentityType::Group.as_str())),
            ),
          ),
      );
      let elements = self.exec.query(query).await?;
      let mut group_id = None;
      let mut key_id = None;
      let mut key_public_key = None;
      let mut key_private_b64: Option<String> = None;

      for el in elements {
        match el {
          Element::Node(node) => {
            if let NodeMeta::Key(KeyMeta {
              public_key,
              private_key,
              ..
            }) = &node.metadata
            {
              log::debug!("Found existing master key with id {}", node.id);
              key_id = Some(node.id);
              key_public_key = Some(public_key.clone());
              key_private_b64 = private_key.clone();
            }
          }
          Element::Edge(edge) => {
            if edge.r#type == EdgeType::Owns.as_str() {
              log::debug!("Found existing master group with id {}", edge.from_id);
              group_id = Some(edge.from_id);
            }
          }
        }
      }

      match (group_id, key_id, key_public_key, key_private_b64) {
        (Some(gid), Some(kid), Some(pk), priv_b64) => Some((gid, kid, pk, priv_b64)),
        _ => {
          log::debug!("No existing master group/key found");
          None
        }
      }
    };

    let (root_group_id, _master_key, master_key_public_key, master_key_created) = match group_node {
      // (group_id, key_id, public_key, optional_private_key_b64)
      Some((group_id, key_id, public_key, _priv_b64)) => (group_id, key_id, public_key, false),
      None => {
        let (master_key, seed_bytes, master_key_created): (Key, Vec<u8>, bool) =
          self.get_or_create_master_key(&options).await?;
        let (_signing_key, verify_key) = master_key.secp256k1_keypair()?;
        let encoded_point = verify_key.to_encoded_point(false);
        let public_key = BASE64_URL_SAFE.encode(encoded_point.as_bytes());

        let secret_b64 = BASE64_URL_SAFE.encode(seed_bytes.as_slice());

        if master_key_created {
          log::debug!("A new master key was created");
        }

        let tx = self.exec.transaction().await?;

        let key_node = tx
          .create_node(
            NodeType::Key.as_str().to_string(),
            NodeMeta::Key(KeyMeta {
              public_key: public_key.clone(),
              private_key: Some(secret_b64),
              derivation_path: master_key.derivation_path(),
            }),
          )
          .await?;
        // Create master group
        let group_node = tx
          .create_node(
            NodeType::Identity.as_str().to_string(),
            NodeMeta::Identity(IdentityMeta::Group {
              name: options
                .root_group_name
                .unwrap_or_else(|| "Everything".to_string()),
              local: true,
            }),
          )
          .await?;
        // Set group OWNS key
        tx.create_edge(
          EdgeType::Owns.as_str().to_string(),
          group_node.id,
          key_node.id,
          EdgeProps::Owns {
            since: options.now,
            until: options.now,
          },
        )
        .await?;

        tx.commit().await?;

        log::debug!(
          "Created master group with id {} and master key with id {}",
          group_node.id,
          key_node.id
        );

        (group_node.id, key_node.id, public_key, master_key_created)
      }
    };

    let identity = IdentityService::new(self.exec.clone());

    let owner_user_id = match identity.find_owner(root_group_id).await? {
      Some(owner_node) => owner_node.id,
      None => {
        // Create the owner user and the OWNS edge in a single transaction so
        // the two-step operation is atomic and cannot leave partial state.
        let tx = self.exec.transaction().await?;

        let user_node = tx
          .create_node(
            NodeType::Identity.as_str().to_string(),
            NodeMeta::Identity(IdentityMeta::User {
              name: options
                .owner_name
                .clone()
                .unwrap_or_else(|| "admin".to_owned()),
              local: true,
            }),
          )
          .await?;

        tx.create_edge(
          EdgeType::Owns.as_str().to_string(),
          user_node.id,
          root_group_id,
          EdgeProps::Owns {
            since: options.now,
            until: options.now,
          },
        )
        .await?;

        tx.commit().await?;

        // DEBUG: indicate owner creation succeeded
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

    let devices = identity.find_root_devices(root_group_id).await?;
    let device_id = if let Some(did_node) = devices.into_iter().next() {
      did_node.id
    } else {
      // Create the device node and MEMBER_OF edge in a single transaction to
      // avoid leaving a device without its group membership on failure.
      let tx = self.exec.transaction().await?;
      let device_node = tx
        .create_node(
          NodeType::Identity.as_str().to_string(),
          NodeMeta::Identity(IdentityMeta::Device {
            name: options.device_name.unwrap_or_else(|| "device".to_string()),
            local: true,
            root: Some(root_group_id),
          }),
        )
        .await?;
      tx.create_edge(
        EdgeType::MemberOf.as_str().to_string(),
        device_node.id,
        root_group_id,
        EdgeProps::MemberOf {
          since: options.now,
          until: options.now,
        },
      )
      .await?;
      tx.commit().await?;
      log::debug!(
        "Created device with id {} belonging to master group {}",
        device_node.id,
        root_group_id
      );
      device_node.id
    };

    Ok(BootstrapResult {
      root_group: root_group_id,
      master_key_public_key,
      master_key_created,
      owner_user: owner_user_id,
      device: device_id,
    })
  }

  /// Return all Key nodes as (id, KeyMeta) pairs.
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
