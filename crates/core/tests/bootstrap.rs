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

struct TestKeyVault;

#[async_trait::async_trait]
impl KeyVault for TestKeyVault {
  async fn get_or_create(&self) -> Result<(Key, Vec<u8>, bool)> {
    // deterministic entropy for test
    let entropy = [0u8; 32];
    let key = Key::from_entropy(&entropy).expect("entropy to key");
    Ok((key, entropy.to_vec(), true))
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
  let kv = TestKeyVault;
  let service = GraphService::new(repo, kv);

  let opts = BootstrapOptions::builder()
    .root_group_name("Everything")
    .build()
    .unwrap();
  let _res = service.bootstrap(opts).await.unwrap();

  // query for Key nodes created without derivation_path
  let query = Query::nodes(
    NodeQuery::new(NodeType::Key.as_str()).filter(!field("metadata.derivation_path").exists()),
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
