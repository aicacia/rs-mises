#![cfg(feature = "in-memory")]

use core::sync::atomic::{AtomicUsize, Ordering};
use std::{ops::RangeBounds, sync::Arc};

use mises_graph::{
  EdgeQuery, Element, Executor, IdGenerator, InMemoryKeyValueRepository, InMemoryKeyValueStore,
  NodeQuery, Query, Repository, Transaction, field,
};

#[derive(Clone)]
struct UsizeIdGenerator {
  counter: Arc<AtomicUsize>,
}

impl UsizeIdGenerator {
  fn new() -> Self {
    Self {
      counter: Arc::new(AtomicUsize::default()),
    }
  }
}

impl IdGenerator<usize> for UsizeIdGenerator {
  fn next(&self) -> usize {
    self.counter.fetch_add(1, Ordering::SeqCst)
  }
}

type Repo =
  InMemoryKeyValueRepository<usize, serde_json::Value, serde_json::Value, UsizeIdGenerator>;

#[tokio::test]
async fn create_and_get_node_edge() {
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());

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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());
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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());
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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());
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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());

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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());

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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());

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

  #[derive(Clone)]
  struct FailingStore {
    inner: std::sync::Arc<InMemoryKeyValueStore>,
    counter: std::sync::Arc<AtomicUsize>,
    fail_on: usize,
  }

  impl FailingStore {
    fn new(fail_on: usize) -> Self {
      Self {
        inner: std::sync::Arc::new(InMemoryKeyValueStore::new()),
        counter: std::sync::Arc::new(AtomicUsize::new(0)),
        fail_on,
      }
    }
  }

  #[async_trait::async_trait]
  impl mises_async_kv_bytes::KeyValueStoreExecutor for FailingStore {
    type Error = mises_graph::GraphError;

    async fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>, Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.get(key).await
    }

    async fn put<K>(&self, key: K, value: Vec<u8>) -> Result<(), Self::Error>
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

    async fn delete<K>(&self, key: K) -> Result<(), Self::Error>
    where
      K: AsRef<[u8]> + Send,
    {
      self.inner.delete(key).await
    }

    async fn scan<R, F>(&self, range: R, f: F) -> Result<(), Self::Error>
    where
      R: RangeBounds<Vec<u8>> + Send,
      F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
    {
      self.inner.scan(range, f).await
    }
  }

  #[async_trait::async_trait]
  impl mises_async_kv_bytes::KeyValueStore for FailingStore {
    type Transaction = <InMemoryKeyValueStore as mises_async_kv_bytes::KeyValueStore>::Transaction;

    async fn transaction(&self) -> Result<Self::Transaction, mises_graph::GraphError> {
      self.inner.transaction().await
    }
  }

  let fail_on_put = 4;
  let store = FailingStore::new(fail_on_put);
  use mises_graph::key_value_repository::KeyValueRepositoryStore;

  #[derive(Clone)]
  struct FailingRepositoryStore {
    store: FailingStore,
    id_gen: UsizeIdGenerator,
  }

  impl KeyValueRepositoryStore for FailingRepositoryStore {
    type Id = usize;
    type NodeMeta = serde_json::Value;
    type EdgeProps = serde_json::Value;
    type Store = FailingStore;
    type IdGen = UsizeIdGenerator;

    fn node_store(&self) -> &Self::Store {
      &self.store
    }

    fn edge_store(&self) -> &Self::Store {
      &self.store
    }

    fn from_index_store(&self) -> &Self::Store {
      &self.store
    }

    fn to_index_store(&self) -> &Self::Store {
      &self.store
    }

    fn id_gen(&self) -> &Self::IdGen {
      &self.id_gen
    }
  }

  let repo = mises_graph::KeyValueRepository::new(FailingRepositoryStore {
    store: store.clone(),
    id_gen: UsizeIdGenerator::new(),
  });

  let n1 = repo
    .create_node("User".into(), serde_json::json!({"name": "A"}))
    .await
    .unwrap();
  let n2 = repo
    .create_node("User".into(), serde_json::json!({"name": "B"}))
    .await
    .unwrap();

  let res = repo
    .create_edge(
      "KNOWS".into(),
      n1.id,
      n2.id,
      serde_json::json!({"since": 2020}),
    )
    .await;
  assert!(res.is_err());

  let q = Query::edges(EdgeQuery::new("KNOWS"));
  let out = repo.query(q).await.unwrap();
  assert_eq!(out.len(), 0);
}

#[tokio::test]
async fn update_node_and_edge_conflict_and_success() {
  use mises_graph::GraphError;
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());

  let n = repo
    .create_node("User".into(), serde_json::json!({"name": "C"}))
    .await
    .unwrap();

  let wrong_time = n.updated_at + chrono::Duration::seconds(1);
  let res = repo
    .update_node(n.id, serde_json::json!({"name": "X"}), Some(wrong_time))
    .await;
  assert!(matches!(res, Err(GraphError::Conflict)));

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

  let current = repo.get_edge_by_id(e.id).await.unwrap().unwrap();
  let wrong_edge_time = current.updated_at + chrono::Duration::seconds(1);
  let eres = repo
    .update_edge(e.id, serde_json::json!({"k": 2}), Some(wrong_edge_time))
    .await;
  assert!(matches!(eres, Err(GraphError::Conflict)));

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
  let repo = Repo::new_in_memory(UsizeIdGenerator::new());
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

  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_some());

  repo.delete_node(a.id).await.unwrap();

  assert!(repo.get_node_by_id(a.id).await.unwrap().is_none());
  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_none());

  assert!(repo.get_node_by_id(b.id).await.unwrap().is_some());
}

#[tokio::test]
async fn delete_node_handles_missing_main_edge_cleanup() {
  use mises_graph::KeyValueStoreExecutor;
  use mises_graph::key_value_repository::KeyValueRepositoryStore;

  // Create a custom store for testing
  #[derive(Clone)]
  struct TestStore {
    node_store: InMemoryKeyValueStore,
    edge_store: InMemoryKeyValueStore,
    from_index_store: InMemoryKeyValueStore,
    to_index_store: InMemoryKeyValueStore,
    id_gen: UsizeIdGenerator,
  }

  impl KeyValueRepositoryStore for TestStore {
    type Id = usize;
    type NodeMeta = serde_json::Value;
    type EdgeProps = serde_json::Value;
    type Store = InMemoryKeyValueStore;
    type IdGen = UsizeIdGenerator;

    fn node_store(&self) -> &Self::Store {
      &self.node_store
    }

    fn edge_store(&self) -> &Self::Store {
      &self.edge_store
    }

    fn from_index_store(&self) -> &Self::Store {
      &self.from_index_store
    }

    fn to_index_store(&self) -> &Self::Store {
      &self.to_index_store
    }

    fn id_gen(&self) -> &Self::IdGen {
      &self.id_gen
    }
  }

  let test_store = TestStore {
    node_store: InMemoryKeyValueStore::new(),
    edge_store: InMemoryKeyValueStore::new(),
    from_index_store: InMemoryKeyValueStore::new(),
    to_index_store: InMemoryKeyValueStore::new(),
    id_gen: UsizeIdGenerator::new(),
  };
  let edge_store = test_store.edge_store().clone();
  let from_index_store = test_store.from_index_store().clone();
  let to_index_store = test_store.to_index_store().clone();
  let repo = mises_graph::KeyValueRepository::new(test_store);

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

  // remove the main edge entry directly from the edge store
  let id_bytes = serde_json::to_vec(&e.id).unwrap();
  edge_store.delete(id_bytes).await.unwrap();

  assert!(repo.get_edge_by_id(e.id).await.unwrap().is_none());

  // ensure index still contains the from entry for the missing main edge
  let mut from_prefix = serde_json::to_vec(&a.id).unwrap();
  from_prefix.push(0);
  let mut matches: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
  from_index_store
    .scan(from_prefix.., |k: &Vec<u8>, v: &Vec<u8>| {
      matches.push((k.clone(), v.clone()));
      true
    })
    .await
    .unwrap();
  assert!(
    matches
      .iter()
      .any(|(_, v)| v == &serde_json::to_vec(&e.id).unwrap())
  );

  repo.delete_node(a.id).await.unwrap();

  // confirm no index entries remain for that edge id
  let mut all_index_entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
  from_index_store
    .scan(vec![].., |k: &Vec<u8>, v: &Vec<u8>| {
      all_index_entries.push((k.clone(), v.clone()));
      true
    })
    .await
    .unwrap();
  to_index_store
    .scan(vec![].., |k: &Vec<u8>, v: &Vec<u8>| {
      all_index_entries.push((k.clone(), v.clone()));
      true
    })
    .await
    .unwrap();
  assert!(
    !all_index_entries
      .iter()
      .any(|(_, v)| v == &serde_json::to_vec(&e.id).unwrap())
  );
}
