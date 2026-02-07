#![cfg(feature = "in-memory")]

use core::sync::atomic::{AtomicUsize, Ordering};

use mises_graph::{
  EdgeQuery, Element, Executor, IdGenerator, KeyValueRepository, NodeQuery, Query, Repository,
  Transaction, field, in_memory_key_value_store::InMemoryKeyValueStore,
};

struct UsizeIdGenerator {
  counter: AtomicUsize,
}

impl UsizeIdGenerator {
  fn new() -> Self {
    Self {
      counter: AtomicUsize::default(),
    }
  }
}

impl IdGenerator<usize> for UsizeIdGenerator {
  fn next(&self) -> usize {
    self.counter.fetch_add(1, Ordering::SeqCst)
  }
}

type Repo = KeyValueRepository<
  usize,
  serde_json::Value,
  serde_json::Value,
  UsizeIdGenerator,
  InMemoryKeyValueStore,
>;

#[tokio::test]
async fn create_and_get_node_edge() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());

  let node = repo
    .create_node("User".into(), serde_json::json!({"name": "Alice"}))
    .await
    .unwrap();
  let got = repo.get_node_by_id(node.id).await.unwrap().unwrap();
  assert_eq!(node, got);

  let node2 = repo
    .create_node("User".into(), serde_json::json!({"name": "Bob"}))
    .await
    .unwrap();
  let edge = repo
    .create_edge(
      "KNOWS".into(),
      node.id,
      node2.id,
      serde_json::json!({"since": 2020}),
    )
    .await
    .unwrap();
  let got_e = repo.get_edge_by_id(edge.id).await.unwrap().unwrap();
  assert_eq!(edge, got_e);
}

#[tokio::test]
async fn transactions_commit_and_rollback() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());
  let tx = repo.transaction().await.unwrap();

  let node = tx
    .create_node("User".into(), serde_json::json!({"name": "T"}))
    .await
    .unwrap();

  tx.commit().await.unwrap();
  assert!(repo.get_node_by_id(node.id).await.unwrap().is_some());

  let tx2 = repo.transaction().await.unwrap();
  let node2 = tx2
    .create_node("User".into(), serde_json::json!({"name": "R"}))
    .await
    .unwrap();
  tx2.rollback().await.unwrap();
  assert!(repo.get_node_by_id(node2.id).await.unwrap().is_none());
}

#[tokio::test]
async fn query_nodes_and_edges() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());
  let n1 = repo
    .create_node("User".into(), serde_json::json!({"name": "A"}))
    .await
    .unwrap();
  let n2 = repo
    .create_node("Key".into(), serde_json::json!({"name": "K"}))
    .await
    .unwrap();
  let _ = repo
    .create_edge(
      "HAS_KEY".into(),
      n1.id,
      n2.id,
      serde_json::json!({"scope": "owner"}),
    )
    .await
    .unwrap();

  let q = Query::nodes(NodeQuery::new("User"));
  let out = repo.query(q).await.unwrap();
  assert_eq!(out.len(), 1);

  let q2 = Query::edges(EdgeQuery::new("HAS_KEY"));
  let out2 = repo.query(q2).await.unwrap();
  assert_eq!(out2.len(), 1);
}

#[tokio::test]
async fn query_with_predicates_and_include_edges() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());
  let key = repo
    .create_node("Key".into(), serde_json::json!({"type": "master"}))
    .await
    .unwrap();
  let group = repo
    .create_node("Identity".into(), serde_json::json!({"type": "group"}))
    .await
    .unwrap();

  let _ = repo
    .create_edge(
      "OWNs".into(),
      group.id,
      key.id,
      serde_json::json!({"scope": "owner"}),
    )
    .await
    .unwrap();

  let q = Query::nodes(
    NodeQuery::new("Key")
      .filter(field("metadata.type").eq("master"))
      .include(
        EdgeQuery::incoming("OWNs")
          .from(NodeQuery::new("Identity").filter(field("metadata.type").eq("group"))),
      ),
  );

  let out = repo.query(q).await.unwrap();

  let mut has_key_node = false;
  let mut has_edge_own = false;
  for el in out {
    match el {
      Element::Node(n) => {
        if n.r#type == "Key"
          && let Some(t) = n.metadata.get("type")
          && t == &serde_json::Value::String("master".into())
        {
          has_key_node = true;
        }
      }
      Element::Edge(e) => {
        if e.r#type == "OWNs" && e.from_id == group.id && e.to_id == key.id {
          has_edge_own = true;
        }
      }
    }
  }

  assert!(has_key_node);
  assert!(has_edge_own);
}

#[tokio::test]
async fn get_json_field_handles_enum_wrapped_metadata() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());

  let n = repo
    .create_node("Key".into(), serde_json::json!({"Key": {"type": "master"}}))
    .await
    .unwrap();

  let q = Query::nodes(NodeQuery::new("Key").filter(field("metadata.type").eq("master")));

  let out = repo.query(q).await.unwrap();
  assert!(
    out
      .iter()
      .any(|el| matches!(el, Element::Node(node) if node.id == n.id))
  );
}

#[tokio::test]
async fn get_json_field_non_object_metadata_no_match() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());

  let n = repo
    .create_node("User".into(), serde_json::json!("just-a-string"))
    .await
    .unwrap();

  let q = Query::nodes(NodeQuery::new("User").filter(field("metadata.type").eq("foo")));

  let out = repo.query(q).await.unwrap();
  assert!(
    !out
      .iter()
      .any(|el| matches!(el, Element::Node(node) if node.id == n.id))
  );
}

#[tokio::test]
async fn exists_predicate_null_not_exists() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());

  let n = repo
    .create_node(
      "Key".into(),
      serde_json::json!({"derivation_path": serde_json::Value::Null}),
    )
    .await
    .unwrap();

  let q = Query::nodes(NodeQuery::new("Key").filter(!field("metadata.derivation_path").exists()));

  let out = repo.query(q).await.unwrap();
  assert!(
    out
      .iter()
      .any(|el| matches!(el, Element::Node(node) if node.id == n.id))
  );
}
