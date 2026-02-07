use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_core::model::{keys::KeyMeta, node::NodeMeta, node::NodeType};
use mises_core::service::graph::KeyVault;
use mises_graph::{Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository};
use mises_key::Key;
use mises_key_vault::GraphKeyVault;
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
async fn graph_keyvault_reads_private_key() {
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
        derivation_path: None,
      }),
    )
    .await
    .unwrap();

  let kv = GraphKeyVault::new(repo);
  let (k, seed_bytes, created) = kv.get_or_create().await.unwrap();
  assert!(!created);
  // seed should match and produce expected secret bytes
  assert_eq!(seed_bytes, [0u8; 32].to_vec());
  let expected = expected;
  assert_eq!(k.secp256k1_secret_bytes().unwrap(), expected);
}
