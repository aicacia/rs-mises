#![cfg(feature = "in-memory")]

use mises_core::model::{keys::KeyMeta, node::NodeMeta, node::NodeType};
use mises_core::service::graph::GraphService;
use mises_graph::{Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository};
use mises_key::Key;
use uuid::Uuid;

#[derive(Clone)]
struct TestUuidGenerator;

impl IdGenerator<Uuid> for TestUuidGenerator {
  fn next(&self) -> Uuid {
    Uuid::new_v4()
  }
}

fn make_repo() -> KeyValueRepository<
  Uuid,
  NodeMeta,
  mises_core::model::edge::EdgeProps,
  TestUuidGenerator,
  InMemoryKeyValueStore,
> {
  KeyValueRepository::new(InMemoryKeyValueStore::new(), TestUuidGenerator)
}

#[tokio::test]
async fn list_keys_returns_keymeta() {
  let repo = make_repo();
  let service = GraphService::new(repo.clone());

  // Create a key and insert into repo
  let key = Key::from_entropy(&[2u8; 32]).expect("key from entropy");
  let km = KeyMeta::from(key);

  let _node = repo
    .create_node(
      NodeType::Key.as_str().to_string(),
      NodeMeta::Key(km.clone()),
    )
    .await
    .expect("create key node");

  // Call list_keys
  let keys = service.list_keys().await.expect("list_keys");

  assert!(!keys.is_empty(), "expected at least one key");

  // Ensure returned KeyMeta decodes and has EC coords
  let found = keys.into_iter().any(|(_, k): (Uuid, KeyMeta)| {
    let b = k.to_bytes();
    let coords = k.ec_coords_b64();
    b.is_ok() && coords.is_some()
  });

  assert!(found, "expected to find a key with valid EC coords");
}
