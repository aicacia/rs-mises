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

#[tokio::test]
async fn in_memory_get_batch_returns_correct_order() {
  use mises_graph::KeyValueStoreExecutor;

  let store = InMemoryKeyValueStore::new();
  // insert a couple of keys
  store.put(b"one".as_ref(), b"1".to_vec()).await.unwrap();
  store.put(b"two".as_ref(), b"2".to_vec()).await.unwrap();

  let keys: Vec<&[u8]> = vec![b"one".as_ref(), b"missing".as_ref(), b"two".as_ref()];
  let results = store.get_batch(keys).await.unwrap();

  assert_eq!(results.len(), 3);
  assert_eq!(results[0].as_deref(), Some(b"1".as_ref()));
  assert!(results[1].is_none());
  assert_eq!(results[2].as_deref(), Some(b"2".as_ref()));
}

#[tokio::test]
async fn create_edge_cleanup_on_partial_failure() {
  use core::sync::atomic::{AtomicUsize, Ordering};
  use mises_graph::{KeyValueStore, KeyValueStoreExecutor};

  // A thin wrapper over InMemoryKeyValueStore that fails on the Nth `put`.
  struct FailingStore {
    inner: InMemoryKeyValueStore,
    counter: AtomicUsize,
    fail_on: usize,
  }

  impl FailingStore {
    fn new(fail_on: usize) -> Self {
      Self {
        inner: InMemoryKeyValueStore::new(),
        counter: AtomicUsize::new(0),
        fail_on,
      }
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStoreExecutor for FailingStore {
    async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, mises_graph::GraphError>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get(key).await
    }

    async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), mises_graph::GraphError>
    where
      K: AsRef<[u8]> + Send,
    {
      let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
      if n == self.fail_on {
        return Err(mises_graph::GraphError::Other(
          "injected failure".to_string().into(),
        ));
      }
      self.inner.put(key, value).await
    }

    async fn delete<K>(&self, key: K) -> Result<(), mises_graph::GraphError>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.delete(key).await
    }

    async fn scan<R, F>(
      &self,
      range: R,
      predicate: F,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, mises_graph::GraphError>
    where
      R: std::ops::RangeBounds<Vec<u8>> + Send,
      F: FnMut(&Vec<u8>, &Vec<u8>) -> Option<bool> + Send,
    {
      self.inner.scan(range, predicate).await
    }
  }

  #[async_trait::async_trait]
  impl KeyValueStore for FailingStore {
    type Transaction = <InMemoryKeyValueStore as KeyValueStore>::Transaction;

    async fn transaction(&self) -> Result<Self::Transaction, mises_graph::GraphError> {
      self.inner.transaction().await
    }
  }

  let store = FailingStore::new(4); // fail on the 4th put (after two nodes + main edge)
  let repo = mises_graph::KeyValueRepository::<
    usize,
    serde_json::Value,
    serde_json::Value,
    UsizeIdGenerator,
    FailingStore,
  >::new(store, UsizeIdGenerator::new());

  let n1 = repo
    .create_node("User".into(), serde_json::json!({"name": "A"}))
    .await
    .unwrap();
  let n2 = repo
    .create_node("User".into(), serde_json::json!({"name": "B"}))
    .await
    .unwrap();

  // Attempt to create an edge; this should return an error and perform
  // best-effort cleanup so no partial edge remains.
  let res = repo
    .create_edge(
      "KNOWS".into(),
      n1.id,
      n2.id,
      serde_json::json!({"since": 2020}),
    )
    .await;
  assert!(res.is_err());

  // Ensure no edge exists after the failed operation.
  let q = Query::edges(EdgeQuery::new("KNOWS"));
  let out = repo.query(q).await.unwrap();
  assert_eq!(out.len(), 0);
}

#[tokio::test]
async fn update_node_and_edge_conflict_and_success() {
  use mises_graph::GraphError;
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());

  // Node update conflict
  let n = repo
    .create_node("User".into(), serde_json::json!({"name": "C"}))
    .await
    .unwrap();

  // Incorrect expected_updated_at -> Conflict
  let wrong_time = n.updated_at + chrono::Duration::seconds(1);
  let res = repo
    .update_node(n.id, serde_json::json!({"name": "X"}), Some(wrong_time))
    .await;
  assert!(matches!(res, Err(GraphError::Conflict)));

  // Correct expected_updated_at -> success
  let before = repo.get_node_by_id(n.id).await.unwrap().unwrap();
  let res2 = repo
    .update_node(
      n.id,
      serde_json::json!({"name": "Y"}),
      Some(before.updated_at),
    )
    .await;
  assert!(res2.is_ok());
  let after = repo.get_node_by_id(n.id).await.unwrap().unwrap();
  assert_eq!(
    after.metadata.get("name").unwrap(),
    &serde_json::Value::String("Y".into())
  );
  assert!(after.updated_at > before.updated_at);

  // Edge update conflict
  let n1 = repo
    .create_node("User".into(), serde_json::json!({"name": "D"}))
    .await
    .unwrap();
  let n2 = repo
    .create_node("User".into(), serde_json::json!({"name": "E"}))
    .await
    .unwrap();

  let e = repo
    .create_edge("LINK".into(), n1.id, n2.id, serde_json::json!({"k": 1}))
    .await
    .unwrap();

  // Fetch current edge to inspect updated_at
  let current = repo.get_edge_by_id(e.id).await.unwrap().unwrap();
  let wrong_edge_time = current.updated_at + chrono::Duration::seconds(1);
  let eres = repo
    .update_edge(e.id, serde_json::json!({"k": 2}), Some(wrong_edge_time))
    .await;
  assert!(matches!(eres, Err(GraphError::Conflict)));

  // Correct expected_updated_at -> success
  let eres2 = repo
    .update_edge(e.id, serde_json::json!({"k": 3}), Some(current.updated_at))
    .await;
  assert!(eres2.is_ok());
  let after_e = repo.get_edge_by_id(e.id).await.unwrap().unwrap();
  assert_eq!(after_e.properties.get("k").unwrap(), &serde_json::json!(3));
  assert!(after_e.updated_at > current.updated_at);
}

#[tokio::test]
async fn delete_node_cascade_removes_edges() {
  let repo = Repo::new(InMemoryKeyValueStore::new(), UsizeIdGenerator::new());
  let a = repo
    .create_node("User".into(), serde_json::json!({"name": "A"}))
    .await
    .unwrap();
  let b = repo
    .create_node("User".into(), serde_json::json!({"name": "B"}))
    .await
    .unwrap();

  let e = repo
    .create_edge(
      "FRIEND".into(),
      a.id,
      b.id,
      serde_json::json!({"since": 2021}),
    )
    .await
    .unwrap();

  // Ensure edge exists
  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_some());

  // Delete node `a` should remove the edge as well
  repo.delete_node(a.id).await.unwrap();

  assert!(repo.get_node_by_id(a.id).await.unwrap().is_none());
  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_none());

  // Other node remains
  assert!(repo.get_node_by_id(b.id).await.unwrap().is_some());
}

#[tokio::test]
async fn delete_node_handles_missing_main_edge_cleanup() {
  use mises_graph::KeyValueStoreExecutor;
  // Create store explicitly so we can manipulate it directly
  let store = InMemoryKeyValueStore::new();
  let repo = Repo::new(store.clone(), UsizeIdGenerator::new());

  let a = repo
    .create_node("User".into(), serde_json::json!({"name": "A"}))
    .await
    .unwrap();
  let b = repo
    .create_node("User".into(), serde_json::json!({"name": "B"}))
    .await
    .unwrap();

  let e = repo
    .create_edge(
      "FRIEND".into(),
      a.id,
      b.id,
      serde_json::json!({"since": 2021}),
    )
    .await
    .unwrap();

  // Simulate corruption: remove main edge record but leave index entries intact
  let mut main_key = b"edge:".to_vec();
  let id_bytes = serde_json::to_vec(&e.id).unwrap();
  main_key.extend_from_slice(&id_bytes);
  store.delete(main_key).await.unwrap();

  // Sanity: main edge gone
  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_none());

  // Ensure from-index entry still exists (pointing to the edge id)
  let mut from_prefix = b"edge_from:".to_vec();
  from_prefix.extend_from_slice(&serde_json::to_vec(&a.id).unwrap());
  from_prefix.push(0);
  let matches = store.scan(from_prefix.., |_, _| Some(true)).await.unwrap();
  assert!(
    matches
      .iter()
      .any(|(_, v)| v == &serde_json::to_vec(&e.id).unwrap())
  );

  // Delete node should not error and should clean up dangling index entries
  repo.delete_node(a.id).await.unwrap();

  // Verify index entries referencing this edge were removed
  let all_from_entries = store
    .scan(b"edge_from:".to_vec().., |_, _| Some(true))
    .await
    .unwrap();
  assert!(
    !all_from_entries
      .iter()
      .any(|(_, v)| v == &serde_json::to_vec(&e.id).unwrap())
  );
}
