#![cfg(feature = "in-memory")]

use mises_core::{
  model::{
    keys::KeyMeta,
    node::{NodeMeta, NodeType},
  },
  service::graph::GraphService,
};
use mises_key::Key;

mod common;

use common::make_repo;

#[tokio::test]
async fn list_keys_returns_keymeta() {
  let repo = make_repo();
  let service = GraphService::new(repo.clone());

  let key = Key::from_entropy(&[2u8; 32]).expect("key from entropy");
  let km = KeyMeta::try_from(key).expect("create keymeta");

  let _node = repo
    .create_node(
      NodeType::Key.as_str().to_string(),
      NodeMeta::Key(km.clone()),
    )
    .await
    .expect("create key node");

  let keys = service.list_keys().await.expect("list_keys");

  assert!(!keys.is_empty(), "expected at least one key");

  let found = keys.into_iter().any(|(_, k): (Uuid, KeyMeta)| {
    let b = k.to_bytes();
    let coords = k.ec_coords_b64();
    b.is_ok() && coords.is_some()
  });

  assert!(found, "expected to find a key with valid EC coords");
}
