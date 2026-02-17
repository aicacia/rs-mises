#![cfg(feature = "in-memory")]

extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use mises_graph::{
  EdgeQuery, Element, Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository, NodeQuery,
  Query, field,
};
use serde_json::json;

#[derive(Clone)]
struct U64Generator(Arc<AtomicU64>);
impl U64Generator {
  fn new() -> Self {
    Self(Arc::new(AtomicU64::new(1)))
  }
}
impl IdGenerator<u64> for U64Generator {
  fn next(&self) -> u64 {
    self.0.fetch_add(1, Ordering::SeqCst)
  }
}

#[tokio::test]
async fn edge_query_filters_by_node_id() {
  let repo: KeyValueRepository<
    u64,
    serde_json::Value,
    serde_json::Value,
    U64Generator,
    InMemoryKeyValueStore,
    InMemoryKeyValueStore,
    InMemoryKeyValueStore,
    InMemoryKeyValueStore,
  > = KeyValueRepository::new(
    mises_graph::InMemoryKeyValueStore::new(),
    mises_graph::InMemoryKeyValueStore::new(),
    mises_graph::InMemoryKeyValueStore::new(),
    mises_graph::InMemoryKeyValueStore::new(),
    U64Generator::new(),
  );

  let n1 = repo
    .create_node("identity".to_string(), json!({ "name": "n1" }))
    .await
    .unwrap();
  let n2 = repo
    .create_node("identity".to_string(), json!({ "name": "n2" }))
    .await
    .unwrap();

  let _edge = repo
    .create_edge("TEST_EDGE".to_string(), n1.id, n2.id, json!({ "at": 1 }))
    .await
    .unwrap();

  let query = Query::edges(
    EdgeQuery::outgoing("TEST_EDGE").from(NodeQuery::any().filter(field("id").eq(n1.id))),
  );

  let all_edges = repo
    .query(Query::edges(EdgeQuery::new("TEST_EDGE".to_string())))
    .await
    .unwrap();
  eprintln!("all edges: {:?}", all_edges);

  let elements = repo.query(query).await.unwrap();
  eprintln!("elements: {:?}", elements);
  let found = elements
    .into_iter()
    .any(|el| matches!(el, Element::Edge(_)));
  assert!(found, "expected to find edge via node id filter");
}
