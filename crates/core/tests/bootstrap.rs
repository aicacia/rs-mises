#![cfg(feature = "in-memory")]

use base64::{Engine, prelude::BASE64_URL_SAFE};
use mises_core::{
  model::{
    keys::{KeyMaterial, KeyMeta},
    node::{NodeMeta, NodeType},
  },
  service::graph::{BootstrapOptions, GraphService},
};
use mises_graph::{Element, Executor, NodeQuery, Query, field};

mod common;

use common::make_repo;

#[tokio::test]
async fn bootstrap_persists_base64_private_key() {
  let repo = make_repo();
  let service = GraphService::new(repo.clone());

  let opts = BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  let _res = service.bootstrap(opts).await.unwrap();

  let query = Query::nodes(
    NodeQuery::new(NodeType::Key.as_str()).filter(field("metadata.derivation_path").eq("m/44'")),
  );

  let elements = repo.query(query).await.unwrap();
  let mut found = false;

  for el in elements {
    if let mises_graph::Element::Node(node) = el
      && let NodeMeta::Key(KeyMeta { private_key, .. }) = &node.metadata
      && private_key.is_some()
    {
      let b64 = private_key.as_ref().unwrap();
      let bytes = BASE64_URL_SAFE.decode(b64.as_bytes()).unwrap();

      let expected_seed = [0u8; 32].to_vec();
      assert_eq!(bytes, expected_seed);
      found = true;
    }
  }

  assert!(found, "expected to find a master key node with private_key");
}

#[tokio::test]
async fn bootstrap_reads_existing_key_node() {
  let repo = make_repo();

  let seed = [0u8; 32];
  let b64 = BASE64_URL_SAFE.encode(seed.as_slice());

  let _ = repo
    .create_node(
      NodeType::Key.as_str().to_string(),
      NodeMeta::Key(KeyMeta {
        public_key: "pub".to_string(),
        private_key: Some(b64.clone()),
        derivation_path: String::from("m/44'"),
        key_material: KeyMaterial::Seed,
      }),
    )
    .await
    .unwrap();

  let service = GraphService::new(repo.clone());
  let opts = BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  let res = service.bootstrap(opts).await.unwrap();
  assert!(!res.master_key_created);

  let all_query = Query::nodes(NodeQuery::new(NodeType::Key.as_str()));
  let elements = repo.query(all_query).await.unwrap();
  let mut found = false;
  for el in elements {
    if let Element::Node(node) = el
      && let NodeMeta::Key(KeyMeta { private_key, .. }) = &node.metadata
      && private_key.as_ref().map(|s| s == &b64).unwrap_or(false)
    {
      found = true;
    }
  }
  assert!(
    found,
    "expected to find a key node with the stored private key"
  );
}
