use mises_core::model::{
  edge::EdgeProps,
  identity::IdentityMeta,
  node::{NodeMeta, NodeType},
  resource::ResourceMeta,
};
use mises_graph::{Executor, InMemoryKeyValueStore, KeyValueRepository, UuidGenerator};
use uuid::Uuid;

pub type Repo = KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>;

pub fn make_repo() -> Repo {
  KeyValueRepository::new(InMemoryKeyValueStore::new(), UuidGenerator::new())
}

pub async fn create_identity(repo: &Repo, meta: IdentityMeta) -> Uuid {
  repo
    .create_node(
      NodeType::Identity.as_str().to_string(),
      NodeMeta::Identity(meta),
    )
    .await
    .unwrap()
    .id
}

pub async fn create_resource(repo: &Repo, r#type: &str) -> Uuid {
  repo
    .create_node(
      NodeType::Resource.as_str().to_string(),
      NodeMeta::Resource(ResourceMeta {
        r#type: r#type.to_string(),
        permissions: vec!["readwrite".to_string()],
      }),
    )
    .await
    .unwrap()
    .id
}
