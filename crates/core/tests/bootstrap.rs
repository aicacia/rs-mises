#![cfg(feature = "in-memory")]

use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_core::{
  CoreError, Result,
  model::{keys::KeyMeta, node::NodeMeta, node::NodeType},
  service::graph::{BootstrapOptions, GraphService, KeyVault},
};
use mises_graph::{
  Element, Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository, NodeQuery, Query,
  field,
};
use mises_key::Key;
use uuid::Uuid;

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
async fn bootstrap_persists_base64_private_key() {
  let repo = make_repo();
  let service = GraphService::new(repo);

  let opts = BootstrapOptions::builder()
    .root_group_name("Everything")
    .test_seed([0u8; 32].to_vec())
    .build()
    .unwrap();
  let _res = service.bootstrap(opts).await.unwrap();

  // query for Key nodes with the master derivation path (m/44')
  let query = Query::nodes(
    NodeQuery::new(NodeType::Key.as_str()).filter(field("metadata.derivation_path").eq("m/44'")),
  );

  let elements = service.repo().query(query).await.unwrap();
  let mut found = false;

  for el in elements {
    if let mises_graph::Element::Node(node) = el {
      if let NodeMeta::Key(KeyMeta { private_key, .. }) = &node.metadata {
        assert!(private_key.is_some());
        let b64 = private_key.as_ref().unwrap();
        let bytes = BASE64_URL_SAFE.decode(b64.as_bytes()).unwrap();

        // compare against expected seed bytes (we store seed bytes directly)
        let expected_seed = [0u8; 32].to_vec();
        assert_eq!(bytes, expected_seed);
        found = true;
      }
    }
  }

  assert!(found, "expected to find a master key node with private_key");
}

#[tokio::test]
async fn bootstrap_reads_existing_key_node() {
  let repo = make_repo();

  // create a deterministic key and persist its seed bytes (base64) in a Key node
  let seed = [0u8; 32];
  let b64 = BASE64_URL_SAFE.encode(seed.as_slice());
  let expected = Key::from(seed.to_vec()).secp256k1_secret_bytes().unwrap();

  let _ = repo
    .create_node(
      NodeType::Key.as_str().to_string(),
      NodeMeta::Key(KeyMeta {
        public_key: "pub".to_string(),
        private_key: Some(b64.clone()),
        derivation_path: String::from("m/44'"),
      }),
    )
    .await
    .unwrap();

  let service = GraphService::new(repo);
  let opts = BootstrapOptions::builder()
    .root_group_name("Everything")
    .build()
    .unwrap();
  let res = service.bootstrap(opts).await.unwrap();
  assert!(!res.master_key_created);

  // verify at least one key node has our base64 private key
  let query = Query::nodes(
    NodeQuery::new(NodeType::Key.as_str()).filter(!field("metadata.derivation_path").exists()),
  );
  let elements = service.repo().query(query).await.unwrap();
  let mut found = false;
  for el in elements {
    if let Element::Node(node) = el {
      if let NodeMeta::Key(KeyMeta { private_key, .. }) = &node.metadata {
        if private_key.as_ref().map(|s| s == &b64).unwrap_or(false) {
          found = true;
        }
      }
    }
  }
  assert!(
    found,
    "expected to find a key node with the stored private key"
  );
}
