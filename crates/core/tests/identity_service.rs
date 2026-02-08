#![cfg(feature = "in-memory")]

use mises_core::model::edge::EdgeProps;
use mises_core::model::identity::{IdentityMeta, IdentityType};
use mises_core::model::node::NodeMeta;
use mises_core::service::identity::IdentityService;
use mises_graph::{Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository};
use uuid::Uuid; // to get create_node

#[derive(Clone)]
struct UuidGenerator;
impl IdGenerator<Uuid> for UuidGenerator {
  fn next(&self) -> Uuid {
    Uuid::new_v4()
  }
}

fn make_repo() -> KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>
{
  KeyValueRepository::new(InMemoryKeyValueStore::new(), UuidGenerator)
}

async fn create_identity(
  repo: &KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>,
  meta: IdentityMeta,
) -> Uuid {
  repo
    .create_node("identity".to_string(), NodeMeta::Identity(meta))
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn get_node_by_id_and_identity_type_happy_path() {
  let repo = make_repo();
  let service = IdentityService::new(repo.clone());

  let id = create_identity(
    &repo,
    IdentityMeta::Application {
      name: "app".to_string(),
      local: true,
      oidc_client: None,
    },
  )
  .await;

  let node = service
    .get_node_by_id_and_identity_type(id, IdentityType::Application)
    .await
    .expect("should find application node");

  assert_eq!(node.id, id);
}

#[tokio::test]
async fn get_node_by_id_and_identity_type_mismatch_and_not_found() {
  let repo = make_repo();
  let service = IdentityService::new(repo.clone());

  let id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "user".to_string(),
      local: true,
    },
  )
  .await;

  let res = service
    .get_node_by_id_and_identity_type(id, IdentityType::Application)
    .await;
  match res {
    Err(e) => match e {
      mises_core::CoreError::InvalidInput(_) => {}
      _ => panic!("expected InvalidInput for mismatched type"),
    },
    Ok(_) => panic!("expected error for mismatched type"),
  }

  let missing = Uuid::new_v4();
  let res = service
    .get_node_by_id_and_identity_type(missing, IdentityType::Application)
    .await;
  match res {
    Err(e) => match e {
      mises_core::CoreError::NotFound => {}
      _ => panic!("expected NotFound for missing node"),
    },
    Ok(_) => panic!("expected error for missing node"),
  }
}
