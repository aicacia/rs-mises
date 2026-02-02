use alloc::{
  boxed::Box,
  string::{String, ToString},
};
use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_URL_SAFE};
use chrono::{DateTime, Utc};
use mises_graph::{EdgeQuery, Element, Executor, Filter, NodeQuery, Query, Transaction, field};
use mises_key::MasterKey;
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
    traits::Repository,
  },
};

#[async_trait]
pub trait KeyVault {
  async fn get_or_create(&self) -> Result<(MasterKey, bool)>;
}

#[derive(Clone)]
pub struct BootstrapOptions {
  pub root_group_name: Option<String>,
  pub owner_name: Option<String>,
  pub device_name: Option<String>,
  pub now: Option<DateTime<Utc>>,
}

impl BootstrapOptions {
  pub fn builder() -> BootstrapOptionsBuilder {
    BootstrapOptionsBuilder {
      root_group_name: None,
      owner_name: None,
      device_name: None,
      now: None,
    }
  }
}

#[derive(Clone, Default)]
pub struct BootstrapOptionsBuilder {
  root_group_name: Option<String>,
  owner_name: Option<String>,
  device_name: Option<String>,
  now: Option<DateTime<Utc>>,
}

impl BootstrapOptionsBuilder {
  pub fn new() -> Self {
    Self {
      root_group_name: None,
      owner_name: None,
      device_name: None,
      now: None,
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

  pub fn build(self) -> Result<BootstrapOptions> {
    Ok(BootstrapOptions {
      root_group_name: self.root_group_name,
      owner_name: self.owner_name,
      device_name: self.device_name,
      now: self.now,
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

#[allow(dead_code)]
pub struct GraphService<R, V>
where
  R: Repository,
  V: KeyVault,
{
  repo: R,
  key_vault: V,
}

impl<R, V> GraphService<R, V>
where
  R: Repository,
  V: KeyVault,
{
  pub fn new(repo: R, key_vault: V) -> Self {
    Self { repo, key_vault }
  }

  pub async fn bootstrap(&self, options: BootstrapOptions) -> Result<BootstrapResult> {
    // 1. Find group that OWNS master key
    let group_node = {
      let query = Query::nodes(
        NodeQuery::new(NodeType::Key.as_str())
          .filter(!field("metadata.derivation_path").exists())
          .include(
            EdgeQuery::incoming(EdgeType::Owns.as_str()).from(
              NodeQuery::new(NodeType::Identity.as_str())
                .filter(field("metadata.type").eq(IdentityType::Group.as_str())),
            ),
          ),
      );
      let elements = self.repo.query(query).await?;
      let mut group_id = None;
      let mut key_id = None;
      let mut key_public_key = None;

      for el in elements {
        match el {
          Element::Node(node) => {
            if let NodeMeta::Key(KeyMeta { public_key, .. }) = &node.metadata {
              log::debug!("Found existing master key with id {}", node.id);
              key_id = Some(node.id);
              key_public_key = Some(public_key.clone());
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

      match (group_id, key_id, key_public_key) {
        (Some(gid), Some(kid), Some(pk)) => Some((gid, kid, pk)),
        _ => {
          log::debug!("No existing master group/key found");
          None
        }
      }
    };

    let (root_group_id, _master_key, master_key_public_key, master_key_created) = match group_node {
      Some((group_id, key_id, public_key)) => (group_id, key_id, public_key, false),
      None => {
        let (master_key, master_key_created) = self.key_vault.get_or_create().await?;
        let (_signing_key, verify_key) = master_key.secp256k1_keypair()?;
        let encoded_point = verify_key.to_encoded_point(false);
        let public_key = BASE64_URL_SAFE.encode(encoded_point.as_bytes());

        if master_key_created {
          log::debug!("A new master key was created");
        }

        let tx = self.repo.transaction().await?;

        // Create master key
        let key_node = tx
          .create_node(
            NodeType::Key.as_str().to_string(),
            NodeMeta::Key(KeyMeta {
              public_key: public_key.clone(),
              derivation_path: None,
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

    // 2. Find user that OWNS the master group
    let owner_user_id = {
      let query = Query::nodes(
        NodeQuery::new(NodeType::Identity.as_str())
          .filter(field("id").eq(root_group_id.to_string()))
          .include(
            EdgeQuery::incoming(EdgeType::Owns.as_str())
              .from(NodeQuery::new(NodeType::Identity.as_str())),
          ),
      );
      let elements = self.repo.query(query).await?;
      let mut user_id = None;

      for el in elements {
        if let Element::Edge(edge) = el
          && edge.r#type == EdgeType::Owns.as_str()
        {
          log::debug!("Found existing owner user with id {}", edge.from_id);
          user_id = Some(edge.from_id);
        }
      }

      if let Some(uid) = user_id {
        uid
      } else {
        let tx = self.repo.transaction().await?;

        // Create admin user
        let user_node = tx
          .create_node(
            NodeType::Identity.as_str().to_string(),
            NodeMeta::Identity(IdentityMeta::User {
              name: options.owner_name.unwrap_or_else(|| "admin".to_string()),
              local: true,
            }),
          )
          .await?;

        tx.create_edge(
          EdgeType::Owns.as_str().to_string(),
          user_node.id,
          root_group_id,
          EdgeProps::Owns {
            since: None,
            until: None,
          },
        )
        .await?;

        tx.commit().await?;

        log::debug!(
          "Created owner user with id {} for master group {}",
          user_node.id,
          root_group_id
        );

        user_node.id
      }
    };

    // 3. Find device that tracks the root group and belongs to the master group
    let device_id = {
      let query = Query::nodes(
        NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
          field("metadata.type")
            .eq(IdentityType::Device.as_str())
            .into(),
          field("metadata.root").eq(root_group_id.to_string()).into(),
        ])),
      );

      let elements = self.repo.query(query).await?;
      let mut found_device = None;

      for el in elements {
        if let Element::Node(node) = el
          && let NodeMeta::Identity(IdentityMeta::Device { .. }) = &node.metadata
        {
          log::debug!("Found existing device with id {}", node.id);
          found_device = Some(node.id);
        }
      }

      if let Some(did) = found_device {
        did
      } else {
        let tx = self.repo.transaction().await?;

        // Create device
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

        // Set device MEMBER_OF master group
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
      }
    };

    Ok(BootstrapResult {
      root_group: root_group_id,
      master_key_public_key,
      master_key_created,
      owner_user: owner_user_id,
      device: device_id,
    })
  }
}
