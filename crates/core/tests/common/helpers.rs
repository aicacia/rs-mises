use uuid::Uuid;

use mises_core::{
  model::{
    edge::EdgeProps,
    identity::IdentityMeta,
    node::{NodeMeta, NodeType},
    resource::ResourceMeta,
  },
  service::{graph::GraphService, identity::IdentityService},
};
use mises_graph::{Executor, InMemoryKeyValueRepository, UuidGenerator};

pub type Repo = InMemoryKeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator>;

pub fn make_repo() -> Repo {
  Repo::new_in_memory(UuidGenerator::new())
}

pub async fn make_bootstrapped_repo() -> Repo {
  let repo = make_repo();
  let graph_service = GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();

  graph_service.bootstrap(opts).await.expect("bootstrap");

  repo
}

pub async fn make_identity_service() -> (Repo, IdentityService<Repo>) {
  let repo = make_bootstrapped_repo().await;
  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  (repo, identity_service)
}

pub async fn create_identity(repo: &Repo, meta: IdentityMeta) -> Uuid {
  repo
    .create_node(
      NodeType::Identity.as_str().to_string(),
      NodeMeta::Identity(Box::new(meta)),
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
