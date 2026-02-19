use alloc::{
  boxed::Box,
  collections::BTreeSet,
  format,
  string::{String, ToString},
  sync::Arc,
  vec,
  vec::Vec,
};

use async_trait::async_trait;
use chrono::Utc;
use hashbrown::{HashMap, HashSet};

use mises_async_kv_bytes::{KeyValueStore, KeyValueStoreExecutor, KeyValueStoreTransaction};

use crate::{
  ComparisonOp, EdgeDirection, EdgeQuery, Filter, NodeQuery, Predicate,
  edge::Edge,
  error::GraphError,
  node::Node,
  query::Query,
  repository::{Executor, Repository, Transaction},
  types::{Element, Id, Value},
};

const SEPARATOR: u8 = 0;

pub trait IdGenerator<I>: Send + Sync {
  fn next(&self) -> I;
}

#[derive(Clone)]
pub struct KeyValueRepository<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  node_store: Arc<SN>,
  edge_store: Arc<SE>,
  from_index_store: Arc<SIF>,
  to_index_store: Arc<SIT>,
  id_gen: Arc<G>,
  _phantom: core::marker::PhantomData<(I, M, P)>,
}

impl<I, M, P, G, SN, SE, SIF, SIT> KeyValueRepository<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  pub fn new(
    node_store: SN,
    edge_store: SE,
    from_index_store: SIF,
    to_index_store: SIT,
    generator: G,
  ) -> Self {
    Self {
      node_store: Arc::new(node_store),
      edge_store: Arc::new(edge_store),
      from_index_store: Arc::new(from_index_store),
      to_index_store: Arc::new(to_index_store),
      id_gen: Arc::new(generator),
      _phantom: core::marker::PhantomData,
    }
  }
}

#[cfg(feature = "in-memory")]
impl<I, M, P, G>
  KeyValueRepository<
    I,
    M,
    P,
    G,
    crate::InMemoryKeyValueStore,
    crate::InMemoryKeyValueStore,
    crate::InMemoryKeyValueStore,
    crate::InMemoryKeyValueStore,
  >
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
{
  /// Convenience constructor for an in-memory repository (all stores are `InMemoryKeyValueStore`).
  pub fn new_in_memory(generator: G) -> Self {
    Self::new(
      crate::InMemoryKeyValueStore::new(),
      crate::InMemoryKeyValueStore::new(),
      crate::InMemoryKeyValueStore::new(),
      crate::InMemoryKeyValueStore::new(),
      generator,
    )
  }
}

pub struct KeyValueTransaction<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  GraphError: From<<SN::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SE::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIF::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIT::Transaction as KeyValueStoreExecutor>::Error>,
  <SN::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SE::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIF::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIT::Transaction as KeyValueStoreExecutor>::Error: Send,
{
  node_tx: SN::Transaction,
  edge_tx: SE::Transaction,
  from_index_tx: SIF::Transaction,
  to_index_tx: SIT::Transaction,
  id_gen: Arc<G>,
  _phantom: core::marker::PhantomData<(I, M, P)>,
}

impl<I, M, P, G, SN, SE, SIF, SIT> KeyValueTransaction<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I> + 'static,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  GraphError: From<<SN::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SE::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIF::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIT::Transaction as KeyValueStoreExecutor>::Error>,
  <SN::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SE::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIF::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIT::Transaction as KeyValueStoreExecutor>::Error: Send,
{
  fn new(
    node_tx: SN::Transaction,
    edge_tx: SE::Transaction,
    from_index_tx: SIF::Transaction,
    to_index_tx: SIT::Transaction,
    id_gen: Arc<G>,
  ) -> Self {
    Self {
      node_tx,
      edge_tx,
      from_index_tx,
      to_index_tx,
      id_gen,
      _phantom: core::marker::PhantomData,
    }
  }
}

#[inline]
fn node_key<I: Id>(id: &I) -> serde_json::Result<Vec<u8>> {
  // node store is dedicated — no global prefix needed
  serde_json::to_vec(id)
}

#[inline]
fn edge_key<I: Id>(id: &I) -> serde_json::Result<Vec<u8>> {
  // edge store is dedicated — no global prefix needed
  serde_json::to_vec(id)
}

#[inline]
fn edge_from_index_key<I: Id>(from_id: &I, edge_id: &I) -> serde_json::Result<Vec<u8>> {
  // index store is dedicated — encode (node_id, edge_id) without a global prefix
  let from_bytes = serde_json::to_vec(from_id)?;
  let edge_bytes = serde_json::to_vec(edge_id)?;
  let mut key = Vec::with_capacity(from_bytes.len() + 1 + edge_bytes.len());
  key.extend_from_slice(&from_bytes);
  key.push(SEPARATOR);
  key.extend_from_slice(&edge_bytes);
  Ok(key)
}

#[inline]
fn edge_to_index_key<I: Id>(to_id: &I, edge_id: &I) -> serde_json::Result<Vec<u8>> {
  let to_bytes = serde_json::to_vec(to_id)?;
  let edge_bytes = serde_json::to_vec(edge_id)?;
  let mut key = Vec::with_capacity(to_bytes.len() + 1 + edge_bytes.len());
  key.extend_from_slice(&to_bytes);
  key.push(SEPARATOR);
  key.extend_from_slice(&edge_bytes);
  Ok(key)
}

#[inline]
fn edge_from_index_prefix<I: Id>(from_id: &I) -> serde_json::Result<Vec<u8>> {
  let from_bytes = serde_json::to_vec(from_id)?;
  let mut key = Vec::with_capacity(from_bytes.len() + 1);
  key.extend_from_slice(&from_bytes);
  key.push(SEPARATOR);
  Ok(key)
}

#[inline]
fn edge_to_index_prefix<I: Id>(to_id: &I) -> serde_json::Result<Vec<u8>> {
  let to_bytes = serde_json::to_vec(to_id)?;
  let mut key = Vec::with_capacity(to_bytes.len() + 1);
  key.extend_from_slice(&to_bytes);
  key.push(SEPARATOR);
  Ok(key)
}

fn prefix_end(prefix: &[u8]) -> Option<Vec<u8>> {
  let mut end = prefix.to_vec();
  for i in (0..end.len()).rev() {
    if end[i] != u8::MAX {
      end[i] += 1;
      end.truncate(i + 1);
      return Some(end);
    }
  }
  None
}

async fn scan_prefix<S, F>(store: &S, prefix: Vec<u8>, f: F) -> Result<(), GraphError>
where
  S: mises_async_kv_bytes::KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
  F: FnMut(&Vec<u8>, &Vec<u8>) -> bool + Send,
{
  match prefix_end(&prefix) {
    Some(end) => store.scan(prefix..end, f).await.map_err(GraphError::from),
    None => store.scan(prefix.., f).await.map_err(GraphError::from),
  }
}

async fn create_node<I, M, G, S>(
  store: &S,
  id_gen: &G,
  r#type: String,
  metadata: M,
) -> Result<Node<I, M>, GraphError>
where
  I: Id,
  M: Value,
  G: IdGenerator<I>,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let id = id_gen.next();
  let now = Utc::now();
  let node = Node {
    id: id.clone(),
    r#type,
    updated_at: now,
    created_at: now,
    metadata,
  };
  let key = node_key(&id)?;
  let value =
    serde_json::to_vec(&node).map_err(|e| GraphError::SerializationError(e.to_string()))?;
  store.put(key, value).await.map_err(GraphError::from)?;
  Ok(node)
}

async fn update_node<I, M, S>(
  store: &S,
  id: I,
  metadata: M,
  expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), GraphError>
where
  I: Id,
  M: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let key = node_key(&id)?;
  let existing: Vec<u8> = store
    .get(&key)
    .await
    .map_err(GraphError::from)?
    .ok_or(GraphError::NotFound)?;
  let mut node: Node<I, M> =
    serde_json::from_slice(&existing).map_err(|e| GraphError::SerializationError(e.to_string()))?;

  if let Some(expected) = expected_updated_at
    && node.updated_at != expected
  {
    return Err(GraphError::Conflict);
  }

  node.metadata = metadata;
  node.updated_at = Utc::now();

  let value =
    serde_json::to_vec(&node).map_err(|e| GraphError::SerializationError(e.to_string()))?;
  store.put(key, value).await.map_err(GraphError::from)?;
  Ok(())
}

async fn delete_node<I, P, SN, SE, SIF, SIT>(
  node_store: &SN,
  edge_store: &SE,
  from_index_store: &SIF,
  to_index_store: &SIT,
  id: I,
) -> Result<(), GraphError>
where
  I: Id,
  P: Value,
  SN: KeyValueStoreExecutor,
  SE: KeyValueStoreExecutor,
  SIF: KeyValueStoreExecutor,
  SIT: KeyValueStoreExecutor,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  let key = node_key(&id)?;
  if node_store.get::<&[u8]>(&key).await?.is_none() {
    return Err(GraphError::NotFound);
  }
  node_store.delete(&key).await.map_err(GraphError::from)?;

  let from_prefix = edge_from_index_prefix(&id)?;
  let to_prefix = edge_to_index_prefix(&id)?;
  let mut edge_ids: BTreeSet<I> = BTreeSet::new();

  let mut index_keys: hashbrown::HashMap<I, Vec<Vec<u8>>> = HashMap::new();

  let mut from_edges = Vec::new();
  scan_prefix(from_index_store, from_prefix, |k, v| {
    if !v.is_empty() {
      from_edges.push((k.clone(), v.clone()));
    }
    true
  })
  .await?;
  for (k, v) in from_edges {
    if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
      edge_ids.insert(edge_id.clone());
      index_keys.entry(edge_id).or_default().push(k);
    }
  }

  let mut to_edges = Vec::new();
  scan_prefix(to_index_store, to_prefix, |k, v| {
    if !v.is_empty() {
      to_edges.push((k.clone(), v.clone()));
    }
    true
  })
  .await?;
  for (k, v) in to_edges {
    if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
      edge_ids.insert(edge_id.clone());
      index_keys.entry(edge_id).or_default().push(k);
    }
  }

  for edge_id in edge_ids {
    match delete_edge::<I, P, SE, SIF, SIT>(
      edge_store,
      from_index_store,
      to_index_store,
      edge_id.clone(),
    )
    .await
    {
      Ok(()) => continue,
      Err(GraphError::NotFound) => {
        if let Some(keys) = index_keys.get(&edge_id) {
          for key in keys {
            let _ = from_index_store.delete(key).await;
            let _ = to_index_store.delete(key).await;
          }
        }

        let edge_id_bytes = match serde_json::to_vec(&edge_id) {
          Ok(b) => b,
          Err(e) => return Err(GraphError::SerializationError(e.to_string())),
        };

        // fallback: remove any index entries whose value equals the edge id (scan both index stores)
        let mut found = Vec::new();
        if scan_prefix(from_index_store, vec![], |k, v| {
          if v == &edge_id_bytes {
            found.push(k.clone());
          }
          true
        })
        .await
        .is_ok()
        {
          for k in found.iter() {
            let _ = from_index_store.delete(k).await;
          }
        }
        let mut found2 = Vec::new();
        if scan_prefix(to_index_store, vec![], |k, v| {
          if v == &edge_id_bytes {
            found2.push(k.clone());
          }
          true
        })
        .await
        .is_ok()
        {
          for k in found2.iter() {
            let _ = to_index_store.delete(k).await;
          }
        }
      }
      Err(e) => return Err(e),
    }
  }

  Ok(())
}

async fn create_edge<I, P, G, SN, SE, SIF, SIT>(
  node_store: &SN,
  edge_store: &SE,
  from_index_store: &SIF,
  to_index_store: &SIT,
  id_gen: &G,
  r#type: String,
  from_id: I,
  to_id: I,
  properties: P,
) -> Result<Edge<I, P>, GraphError>
where
  I: Id,
  P: Value,
  G: IdGenerator<I>,
  SN: KeyValueStoreExecutor,
  SE: KeyValueStoreExecutor,
  SIF: KeyValueStoreExecutor,
  SIT: KeyValueStoreExecutor,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  let from_key = node_key(&from_id)?;
  let to_key = node_key(&to_id)?;

  if node_store.get::<&[u8]>(&from_key).await?.is_none()
    || node_store.get::<&[u8]>(&to_key).await?.is_none()
  {
    return Err(GraphError::NotFound);
  }

  let id = id_gen.next();
  let now = Utc::now();
  let edge = Edge {
    id: id.clone(),
    r#type,
    from_id,
    to_id,
    updated_at: now,
    created_at: now,
    properties,
  };

  let key = edge_key(&id)?;
  let value =
    serde_json::to_vec(&edge).map_err(|e| GraphError::SerializationError(e.to_string()))?;

  edge_store
    .put(key.clone(), value)
    .await
    .map_err(GraphError::from)?;

  let edge_id_bytes =
    serde_json::to_vec(&edge.id).map_err(|e| GraphError::SerializationError(e.to_string()))?;
  let from_index_key = edge_from_index_key(&edge.from_id, &edge.id)?;
  let to_index_key = edge_to_index_key(&edge.to_id, &edge.id)?;

  if let Err(e) = from_index_store
    .put(from_index_key.clone(), edge_id_bytes.clone())
    .await
  {
    let _ = edge_store.delete(&key).await;
    return Err(GraphError::from(e));
  }

  if let Err(e) = to_index_store
    .put(to_index_key.clone(), edge_id_bytes.clone())
    .await
  {
    let _ = edge_store.delete(&key).await;
    let _ = from_index_store.delete(&from_index_key).await;
    return Err(GraphError::from(e));
  }

  Ok(edge)
}

async fn update_edge<I, P, S>(
  store: &S,
  id: I,
  properties: P,
  expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), GraphError>
where
  I: Id,
  P: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let key = edge_key(&id)?;
  let existing = store
    .get(&key)
    .await
    .map_err(GraphError::from)?
    .ok_or(GraphError::NotFound)?;
  let mut edge: Edge<I, P> =
    serde_json::from_slice(&existing).map_err(|e| GraphError::SerializationError(e.to_string()))?;

  if let Some(expected) = expected_updated_at
    && edge.updated_at != expected
  {
    return Err(GraphError::Conflict);
  }

  edge.properties = properties;
  edge.updated_at = Utc::now();

  let value =
    serde_json::to_vec(&edge).map_err(|e| GraphError::SerializationError(e.to_string()))?;
  store.put(key, value).await.map_err(GraphError::from)?;
  Ok(())
}

async fn delete_edge<I, P, SE, SIF, SIT>(
  edge_store: &SE,
  from_index_store: &SIF,
  to_index_store: &SIT,
  id: I,
) -> Result<(), GraphError>
where
  I: Id,
  P: Value,
  SE: KeyValueStoreExecutor,
  SIF: KeyValueStoreExecutor,
  SIT: KeyValueStoreExecutor,
  GraphError: From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  let key = edge_key(&id)?;
  let existing = edge_store
    .get(&key)
    .await
    .map_err(GraphError::from)?
    .ok_or(GraphError::NotFound)?;
  let edge: Edge<I, P> =
    serde_json::from_slice(&existing).map_err(|e| GraphError::SerializationError(e.to_string()))?;
  let from_index_key = edge_from_index_key(&edge.from_id, &edge.id)?;
  let to_index_key = edge_to_index_key(&edge.to_id, &edge.id)?;
  from_index_store
    .delete(&from_index_key)
    .await
    .map_err(GraphError::from)?;
  to_index_store
    .delete(&to_index_key)
    .await
    .map_err(GraphError::from)?;
  edge_store.delete(&key).await.map_err(GraphError::from)?;
  Ok(())
}
async fn get_node_by_id<I, M, S>(store: &S, id: I) -> Result<Option<Node<I, M>>, GraphError>
where
  I: Id,
  M: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let key = node_key(&id)?;
  match store.get::<&[u8]>(&key).await? {
    Some(data) => {
      let node =
        serde_json::from_slice(&data).map_err(|e| GraphError::SerializationError(e.to_string()))?;
      Ok(Some(node))
    }
    None => Ok(None),
  }
}

async fn get_edge_by_id<I, P, S>(store: &S, id: I) -> Result<Option<Edge<I, P>>, GraphError>
where
  I: Id,
  P: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let key = edge_key(&id)?;
  match store.get::<&[u8]>(&key).await? {
    Some(data) => {
      let edge =
        serde_json::from_slice(&data).map_err(|e| GraphError::SerializationError(e.to_string()))?;
      Ok(Some(edge))
    }
    None => Ok(None),
  }
}

async fn get_nodes_batch<I, M, S>(
  store: &S,
  ids: &[I],
) -> Result<HashMap<I, Node<I, M>>, GraphError>
where
  I: Id,
  M: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let mut keys: Vec<Vec<u8>> = Vec::with_capacity(ids.len());
  for id in ids {
    keys.push(node_key(id)?);
  }

  let results: Vec<Option<Vec<u8>>> = store.get_batch(keys).await?;
  let mut nodes = HashMap::with_capacity(ids.len());

  for (idx, maybe_data) in results.into_iter().enumerate() {
    if let Some(data) = maybe_data {
      let node: Node<I, M> =
        serde_json::from_slice(&data).map_err(|e| GraphError::SerializationError(e.to_string()))?;
      nodes.insert(ids[idx].clone(), node);
    }
  }

  Ok(nodes)
}

fn get_json_field<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
  let mut cur = v;
  for part in path.split('.') {
    match cur {
      serde_json::Value::Object(map) => {
        if let Some(next) = map.get(part) {
          cur = next;
        } else if map.len() == 1 {
          if let Some(sole_value) = map.values().next() {
            if let Some(obj) = sole_value.as_object() {
              if let Some(next) = obj.get(part) {
                cur = next;
              } else {
                return None;
              }
            } else {
              return None;
            }
          }
        } else {
          return None;
        }
      }
      _ => return None,
    }
  }
  Some(cur)
}

fn strip_field_prefix_in_predicate(p: &Predicate, prefix: &str) -> Predicate {
  let mut p2 = p.clone();
  let pat = format!("{}.", prefix);
  if p2.field.starts_with(&pat) {
    p2.field = p2.field[pat.len()..].to_string();
  }
  p2
}

fn strip_field_prefix_in_filter(f: &Filter, prefix: &str) -> Filter {
  match f {
    Filter::Predicate(p) => Filter::Predicate(strip_field_prefix_in_predicate(p, prefix)),
    Filter::And(vec) => Filter::And(
      vec
        .iter()
        .map(|ff| strip_field_prefix_in_filter(ff, prefix))
        .collect(),
    ),
    Filter::Or(vec) => Filter::Or(
      vec
        .iter()
        .map(|ff| strip_field_prefix_in_filter(ff, prefix))
        .collect(),
    ),
    Filter::Not(inner) => Filter::Not(Box::new(strip_field_prefix_in_filter(inner, prefix))),
  }
}

#[inline]
fn compare_json(a: &serde_json::Value, op: &ComparisonOp, b: &serde_json::Value) -> bool {
  match op {
    ComparisonOp::Eq => a == b,
    ComparisonOp::Ne => a != b,
    ComparisonOp::In => match b {
      serde_json::Value::Array(values) => values.iter().any(|v| v == a),
      _ => false,
    },
    ComparisonOp::Contains => match a {
      serde_json::Value::Array(values) => values.iter().any(|v| v == b),
      serde_json::Value::String(s) => match b.as_str() {
        Some(sub) => s.contains(sub),
        None => false,
      },
      _ => false,
    },
    ComparisonOp::Gt | ComparisonOp::Gte | ComparisonOp::Lt | ComparisonOp::Lte => {
      if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
        return match op {
          ComparisonOp::Gt => af > bf,
          ComparisonOp::Gte => af >= bf,
          ComparisonOp::Lt => af < bf,
          ComparisonOp::Lte => af <= bf,
          _ => false,
        };
      }
      if let (Some(as_), Some(bs_)) = (a.as_str(), b.as_str()) {
        return match op {
          ComparisonOp::Gt => as_ > bs_,
          ComparisonOp::Gte => as_ >= bs_,
          ComparisonOp::Lt => as_ < bs_,
          ComparisonOp::Lte => as_ <= bs_,
          _ => false,
        };
      }
      false
    }
    _ => false,
  }
}

#[inline]
fn eval_predicate_on_json(v: &serde_json::Value, p: &Predicate) -> bool {
  match p.op {
    ComparisonOp::Exists => get_json_field(v, &p.field).is_some_and(|val| !val.is_null()),
    _ => {
      if let Some(fv) = get_json_field(v, &p.field) {
        if let Some(ref pv) = p.value {
          compare_json(fv, &p.op, pv)
        } else {
          false
        }
      } else {
        false
      }
    }
  }
}

#[inline]
fn eval_filter_on_json(v: &serde_json::Value, f: &Filter) -> bool {
  match f {
    Filter::Predicate(p) => eval_predicate_on_json(v, p),
    Filter::And(vec) => vec.iter().all(|ff| eval_filter_on_json(v, ff)),
    Filter::Or(vec) => vec.iter().any(|ff| eval_filter_on_json(v, ff)),
    Filter::Not(boxed) => !eval_filter_on_json(v, boxed),
  }
}

#[inline]
fn node_matches_query<I, M>(n: &Node<I, M>, nq: &NodeQuery) -> bool
where
  I: Id,
  M: Value,
{
  if let Some(t) = &nq.node_type
    && &n.r#type != t
  {
    return false;
  }
  if let Some(filter) = &nq.filter {
    let filter = strip_field_prefix_in_filter(filter, "metadata");
    let meta_json = serde_json::to_value(&n.metadata).unwrap_or(serde_json::Value::Null);
    if eval_filter_on_json(&meta_json, &filter) {
    } else {
      let node_json = serde_json::to_value(n).unwrap_or(serde_json::Value::Null);
      if !eval_filter_on_json(&node_json, &filter) {
        return false;
      }
    }
  }
  true
}

fn edge_matches_query<I, M, P>(nodes: &[Node<I, M>], e: &Edge<I, P>, eq: &EdgeQuery) -> bool
where
  I: Id,
  M: Value,
  P: Value,
{
  if let Some(t) = &eq.edge_type
    && &e.r#type != t
  {
    return false;
  }

  if let Some(filter) = &eq.filter {
    let filter = strip_field_prefix_in_filter(filter, "properties");
    let json = serde_json::to_value(&e.properties).unwrap_or(serde_json::Value::Null);
    if !eval_filter_on_json(&json, &filter) {
      return false;
    }
  }

  if let Some(fq) = &eq.from {
    if let Some(from_node) = nodes.iter().find(|n| n.id == e.from_id) {
      if !node_matches_query::<I, M>(from_node, fq) {
        return false;
      }
    } else {
      return false;
    }
  }
  if let Some(tq) = &eq.to {
    if let Some(to_node) = nodes.iter().find(|n| n.id == e.to_id) {
      if !node_matches_query::<I, M>(to_node, tq) {
        return false;
      }
    } else {
      return false;
    }
  }
  true
}

async fn collect_node_ids_for_query<I, M, S>(
  store: &S,
  nq: &NodeQuery,
) -> Result<BTreeSet<I>, GraphError>
where
  I: Id,
  M: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let max_nodes = nq.options.limit.unwrap_or(usize::MAX);
  let enforce_limit = max_nodes != usize::MAX;
  let mut node_count = 0;
  let mut ids = BTreeSet::new();

  // scan entire dedicated node store
  scan_prefix(store, vec![], |_, data| {
    if enforce_limit && node_count >= max_nodes {
      return false;
    }

    if let Ok(node) = serde_json::from_slice::<Node<I, M>>(data)
      && node_matches_query::<I, M>(&node, nq)
    {
      if enforce_limit {
        node_count += 1;
      }
      ids.insert(node.id);
    }

    true
  })
  .await?;

  Ok(ids)
}

async fn collect_edge_ids_by_node_ids<I, S>(
  store: &S,
  node_ids: &BTreeSet<I>,
  use_from: bool,
) -> Result<BTreeSet<I>, GraphError>
where
  I: Id,
  S: mises_async_kv_bytes::KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let mut edge_ids = BTreeSet::new();

  for node_id in node_ids {
    let prefix = if use_from {
      edge_from_index_prefix(node_id)?
    } else {
      edge_to_index_prefix(node_id)?
    };

    let mut edges = Vec::new();
    scan_prefix(store, prefix, |k, v| {
      if !v.is_empty() {
        edges.push((k.clone(), v.clone()));
      }
      true
    })
    .await?;
    for (_, v) in edges {
      if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
        edge_ids.insert(edge_id);
      }
    }
  }

  Ok(edge_ids)
}

async fn get_nodes_for_edge_match<I, M, P, S>(
  store: &S,
  edge: &Edge<I, P>,
  eq: &EdgeQuery,
  cache: &mut HashMap<I, Node<I, M>>,
) -> Result<Vec<Node<I, M>>, GraphError>
where
  I: Id,
  M: Value,
  P: Value,
  S: KeyValueStoreExecutor,
  GraphError: From<S::Error>,
  S::Error: Send,
{
  let mut nodes_needed = Vec::with_capacity(2);
  let mut ids_to_fetch = Vec::new();

  if eq.from.is_none() && eq.to.is_none() {
    return Ok(nodes_needed);
  }

  if !cache.contains_key(&edge.from_id) {
    ids_to_fetch.push(edge.from_id.clone());
  }
  if !cache.contains_key(&edge.to_id) && edge.from_id != edge.to_id {
    ids_to_fetch.push(edge.to_id.clone());
  }

  if !ids_to_fetch.is_empty() {
    let fetched = get_nodes_batch::<I, M, S>(store, &ids_to_fetch).await?;
    cache.extend(fetched);
  }

  if let Some(node) = cache.get(&edge.from_id) {
    nodes_needed.push(node.clone());
  }
  if edge.from_id != edge.to_id
    && let Some(node) = cache.get(&edge.to_id)
  {
    nodes_needed.push(node.clone());
  }

  Ok(nodes_needed)
}

async fn edges_by_node<I, P, SE, SIF, SIT>(
  edge_store: &SE,
  from_index_store: &SIF,
  to_index_store: &SIT,
  node_id: &I,
  direction: EdgeDirection,
) -> Result<Vec<Edge<I, P>>, GraphError>
where
  I: Id,
  P: Value,
  SE: mises_async_kv_bytes::KeyValueStoreExecutor,
  SIF: mises_async_kv_bytes::KeyValueStoreExecutor,
  SIT: mises_async_kv_bytes::KeyValueStoreExecutor,
  GraphError: From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  let mut edge_ids = Vec::new();

  match direction {
    EdgeDirection::Out => {
      let from_prefix = edge_from_index_prefix(node_id)?;
      let mut from_edges = Vec::new();
      scan_prefix(from_index_store, from_prefix, |k, v| {
        if !v.is_empty() {
          from_edges.push((k.clone(), v.clone()));
        }
        true
      })
      .await?;
      for (_, v) in from_edges {
        if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
          edge_ids.push(edge_id);
        }
      }
    }
    EdgeDirection::In => {
      let to_prefix = edge_to_index_prefix(node_id)?;
      let mut to_edges = Vec::new();
      scan_prefix(to_index_store, to_prefix, |k, v| {
        if !v.is_empty() {
          to_edges.push((k.clone(), v.clone()));
        }
        true
      })
      .await?;
      for (_, v) in to_edges {
        if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
          edge_ids.push(edge_id);
        }
      }
    }
    EdgeDirection::Both => {
      let from_prefix = edge_from_index_prefix(node_id)?;
      let mut from_edges = Vec::new();
      scan_prefix(from_index_store, from_prefix, |k, v| {
        if !v.is_empty() {
          from_edges.push((k.clone(), v.clone()));
        }
        true
      })
      .await?;
      let mut seen = HashSet::new();
      for (_, v) in from_edges {
        if let Ok(edge_id) = serde_json::from_slice::<I>(&v) {
          edge_ids.push(edge_id.clone());
          let _ = seen.insert(edge_id);
        }
      }

      let to_prefix = edge_to_index_prefix(node_id)?;
      let mut to_edges = Vec::new();
      scan_prefix(to_index_store, to_prefix, |k, v| {
        if !v.is_empty() {
          to_edges.push((k.clone(), v.clone()));
        }
        true
      })
      .await?;
      for (_, v) in to_edges {
        if let Ok(edge_id) = serde_json::from_slice::<I>(&v)
          && seen.insert(edge_id.clone())
        {
          edge_ids.push(edge_id);
        }
      }
    }
  }

  let mut edges = Vec::with_capacity(edge_ids.len());
  for edge_id in edge_ids {
    if let Ok(Some(edge)) = get_edge_by_id::<I, P, SE>(edge_store, edge_id).await {
      edges.push(edge);
    }
  }
  Ok(edges)
}

#[inline]
fn should_stop_collecting(out_len: usize, global_limit: Option<usize>) -> bool {
  if let Some(limit) = global_limit {
    out_len >= limit
  } else {
    false
  }
}

async fn run_query<I, M, P, SN, SE, SIF, SIT>(
  node_store: &SN,
  edge_store: &SE,
  from_index_store: &SIF,
  to_index_store: &SIT,
  query: Query,
) -> Result<Vec<Element<Node<I, M>, Edge<I, P>>>, GraphError>
where
  I: Id,
  M: Value,
  P: Value,
  SN: KeyValueStoreExecutor,
  SE: KeyValueStoreExecutor,
  SIF: KeyValueStoreExecutor,
  SIT: KeyValueStoreExecutor,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  let mut out = Vec::new();
  let global_limit = query.options.limit;

  if let Some(qn) = query.node {
    let node_limit = qn.options.limit;
    let max_nodes = node_limit.unwrap_or(usize::MAX);
    let mut node_count = 0;
    let enforce_limit = max_nodes != usize::MAX;
    let mut node_cache: HashMap<I, Node<I, M>> = HashMap::new();

    let mut nodes = Vec::with_capacity(core::cmp::min(16, max_nodes));

    // scan the dedicated node store (no in-store prefix required)
    scan_prefix(node_store, vec![], |_, data| {
      if enforce_limit && node_count >= max_nodes {
        return false;
      }

      if let Ok(node) = serde_json::from_slice::<Node<I, M>>(data)
        && node_matches_query::<I, M>(&node, &qn)
      {
        if enforce_limit {
          node_count += 1;
        }
        nodes.push(node);
      }

      true
    })
    .await?;

    for n in nodes {
      if should_stop_collecting(out.len(), global_limit) {
        break;
      }

      let _ = node_cache.insert(n.id.clone(), n.clone());

      if !qn.include_edges.is_empty() {
        for ie in &qn.include_edges {
          if should_stop_collecting(out.len(), global_limit) {
            break;
          }

          let edges = edges_by_node::<I, P, SE, SIF, SIT>(
            edge_store,
            from_index_store,
            to_index_store,
            &n.id,
            ie.direction.clone(),
          )
          .await?;

          for e in edges {
            if should_stop_collecting(out.len(), global_limit) {
              break;
            }

            if !matches!(ie.direction, EdgeDirection::Both)
              && match ie.direction {
                EdgeDirection::Out => e.from_id != n.id,
                EdgeDirection::In => e.to_id != n.id,
                EdgeDirection::Both => false,
              }
            {
              continue;
            }

            let mut nodes_for_match = vec![n.clone()];
            let other_id = if e.from_id == n.id {
              &e.to_id
            } else {
              &e.from_id
            };
            if other_id != &n.id {
              if let Some(other_node) = node_cache.get(other_id) {
                nodes_for_match.push(other_node.clone());
              } else if let Ok(Some(other_node)) =
                get_node_by_id::<I, M, SN>(node_store, other_id.clone()).await
              {
                let _ = node_cache.insert(other_node.id.clone(), other_node.clone());
                nodes_for_match.push(other_node);
              }
            }

            if edge_matches_query(nodes_for_match.as_slice(), &e, ie) {
              out.push(Element::Edge(e));
            }
          }
        }
        out.push(Element::Node(n));
      } else {
        out.push(Element::Node(n));
      }
    }
  } else if let Some(qe) = query.edge {
    let edge_limit = qe.options.limit;
    let max_edges = edge_limit.unwrap_or(usize::MAX);
    let mut edge_count = 0;
    let edge_requires_nodes = qe.from.is_some() || qe.to.is_some();
    let effective_limit = match global_limit {
      Some(limit) => core::cmp::min(limit, max_edges),
      None => max_edges,
    };
    let mut node_cache: HashMap<I, Node<I, M>> = HashMap::new();

    if edge_requires_nodes {
      if effective_limit == 0 {
        return Ok(out);
      }

      let from_ids = match &qe.from {
        Some(from_q) => Some(collect_node_ids_for_query::<I, M, SN>(node_store, from_q).await?),
        None => None,
      };
      let to_ids = match &qe.to {
        Some(to_q) => Some(collect_node_ids_for_query::<I, M, SN>(node_store, to_q).await?),
        None => None,
      };

      if from_ids.as_ref().is_some_and(BTreeSet::is_empty)
        || to_ids.as_ref().is_some_and(BTreeSet::is_empty)
      {
        return Ok(out);
      }

      let edge_ids = match (from_ids.as_ref(), to_ids.as_ref()) {
        (Some(from_set), Some(to_set)) => {
          let mut from_edges =
            collect_edge_ids_by_node_ids::<I, SIF>(from_index_store, from_set, true).await?;
          let to_edges =
            collect_edge_ids_by_node_ids::<I, SIT>(to_index_store, to_set, false).await?;
          from_edges.retain(|edge_id| to_edges.contains(edge_id));
          from_edges
        }
        (Some(from_set), None) => {
          collect_edge_ids_by_node_ids::<I, SIF>(from_index_store, from_set, true).await?
        }
        (None, Some(to_set)) => {
          collect_edge_ids_by_node_ids::<I, SIT>(to_index_store, to_set, false).await?
        }
        (None, None) => BTreeSet::new(),
      };

      for edge_id in edge_ids {
        if edge_count >= max_edges {
          break;
        }
        if should_stop_collecting(out.len(), global_limit) {
          break;
        }

        if let Ok(Some(edge)) = get_edge_by_id::<I, P, SE>(edge_store, edge_id).await {
          if let Some(t) = &qe.edge_type
            && &edge.r#type != t
          {
            continue;
          }

          if let Some(filter) = &qe.filter {
            let filter = strip_field_prefix_in_filter(filter, "properties");
            let json = serde_json::to_value(&edge.properties).unwrap_or(serde_json::Value::Null);
            if !eval_filter_on_json(&json, &filter) {
              continue;
            }
          }

          let nodes_for_match =
            get_nodes_for_edge_match::<I, M, P, SN>(node_store, &edge, &qe, &mut node_cache)
              .await?;

          if !edge_matches_query(nodes_for_match.as_slice(), &edge, &qe) {
            continue;
          }

          out.push(Element::Edge(edge));
          edge_count += 1;
        }
      }
    } else {
      let mut prefilter_count = 0;
      let mut edges = Vec::new();
      // scan the dedicated edge store (no in-store prefix required)
      scan_prefix(edge_store, vec![], |_, data| {
        let edge = match serde_json::from_slice::<Edge<I, P>>(data) {
          Ok(edge) => edge,
          Err(_) => return true,
        };

        if let Some(t) = &qe.edge_type
          && &edge.r#type != t
        {
          return true;
        }

        if let Some(filter) = &qe.filter {
          let json = serde_json::to_value(&edge.properties).unwrap_or(serde_json::Value::Null);
          if !eval_filter_on_json(&json, filter) {
            return true;
          }
        }

        if effective_limit == 0 {
          return false;
        }
        if prefilter_count >= effective_limit {
          return false;
        }
        prefilter_count += 1;
        edges.push(edge);

        true
      })
      .await?;

      for edge in edges {
        if edge_count >= max_edges {
          break;
        }
        if should_stop_collecting(out.len(), global_limit) {
          break;
        }

        let nodes_for_match =
          get_nodes_for_edge_match::<I, M, P, SN>(node_store, &edge, &qe, &mut node_cache).await?;

        if !edge_matches_query(nodes_for_match.as_slice(), &edge, &qe) {
          continue;
        }

        out.push(Element::Edge(edge));
        edge_count += 1;
      }
    }
  }

  if let Some(limit) = global_limit {
    out.truncate(limit);
  }

  Ok(out)
}
#[async_trait]
impl<I, M, P, G, SN, SE, SIF, SIT> Executor for KeyValueRepository<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I>,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
{
  type Id = I;

  type NodeMeta = M;
  type EdgeProps = P;

  type Node = Node<I, M>;
  type Edge = Edge<I, P>;

  async fn create_node(
    &self,
    r#type: String,
    metadata: Self::NodeMeta,
  ) -> Result<Self::Node, GraphError> {
    create_node::<I, M, G, _>(&*self.node_store, &*self.id_gen, r#type, metadata).await
  }

  async fn update_node(
    &self,
    id: Self::Id,
    metadata: Self::NodeMeta,
    expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), GraphError> {
    update_node::<I, M, _>(&*self.node_store, id, metadata, expected_updated_at).await
  }

  async fn delete_node(&self, id: Self::Id) -> Result<(), GraphError> {
    delete_node::<I, P, _, _, _, _>(
      &*self.node_store,
      &*self.edge_store,
      &*self.from_index_store,
      &*self.to_index_store,
      id,
    )
    .await
  }

  async fn create_edge(
    &self,
    r#type: String,
    from_id: Self::Id,
    to_id: Self::Id,
    properties: Self::EdgeProps,
  ) -> Result<Self::Edge, GraphError> {
    create_edge::<I, P, G, _, _, _, _>(
      &*self.node_store,
      &*self.edge_store,
      &*self.from_index_store,
      &*self.to_index_store,
      &*self.id_gen,
      r#type,
      from_id,
      to_id,
      properties,
    )
    .await
  }

  async fn update_edge(
    &self,
    id: Self::Id,
    properties: Self::EdgeProps,
    expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), GraphError> {
    update_edge::<I, P, _>(&*self.edge_store, id, properties, expected_updated_at).await
  }

  async fn delete_edge(&self, id: Self::Id) -> Result<(), GraphError> {
    delete_edge::<I, P, _, _, _>(
      &*self.edge_store,
      &*self.from_index_store,
      &*self.to_index_store,
      id,
    )
    .await
  }

  async fn get_node_by_id(&self, id: Self::Id) -> Result<Option<Self::Node>, GraphError> {
    get_node_by_id::<I, M, _>(&*self.node_store, id).await
  }

  async fn get_edge_by_id(&self, id: Self::Id) -> Result<Option<Self::Edge>, GraphError> {
    get_edge_by_id::<I, P, _>(&*self.edge_store, id).await
  }

  async fn query(&self, query: Query) -> Result<Vec<Element<Self::Node, Self::Edge>>, GraphError> {
    run_query::<I, M, P, _, _, _, _>(
      &*self.node_store,
      &*self.edge_store,
      &*self.from_index_store,
      &*self.to_index_store,
      query,
    )
    .await
  }
}

#[async_trait]
impl<I, M, P, G, SN, SE, SIF, SIT> Transaction for KeyValueTransaction<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I>,
  SN: KeyValueStore + 'static,
  SE: KeyValueStore + 'static,
  SIF: KeyValueStore + 'static,
  SIT: KeyValueStore + 'static,
  SN::Transaction: KeyValueStoreTransaction + 'static,
  SE::Transaction: KeyValueStoreTransaction + 'static,
  SIF::Transaction: KeyValueStoreTransaction + 'static,
  SIT::Transaction: KeyValueStoreTransaction + 'static,
  GraphError: From<<SN::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SE::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIF::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIT::Transaction as KeyValueStoreExecutor>::Error>,
  <SN::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SE::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIF::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIT::Transaction as KeyValueStoreExecutor>::Error: Send,
{
  async fn commit(mut self) -> Result<(), GraphError> {
    // attempt to commit all sub-transactions; return first error encountered
    self.node_tx.commit().await.map_err(GraphError::from)?;
    self.edge_tx.commit().await.map_err(GraphError::from)?;
    self
      .from_index_tx
      .commit()
      .await
      .map_err(GraphError::from)?;
    self.to_index_tx.commit().await.map_err(GraphError::from)?;
    Ok(())
  }

  async fn rollback(mut self) -> Result<(), GraphError> {
    // attempt to rollback all sub-transactions; return first error encountered
    let mut first_err: Option<GraphError> = None;
    if let Err(e) = self.node_tx.rollback().await.map_err(GraphError::from) {
      first_err = Some(e);
    }
    if let Err(e) = self.edge_tx.rollback().await.map_err(GraphError::from)
      && first_err.is_none()
    {
      first_err = Some(e);
    }
    if let Err(e) = self
      .from_index_tx
      .rollback()
      .await
      .map_err(GraphError::from)
      && first_err.is_none()
    {
      first_err = Some(e);
    }
    if let Err(e) = self.to_index_tx.rollback().await.map_err(GraphError::from)
      && first_err.is_none()
    {
      first_err = Some(e);
    }
    if let Some(e) = first_err {
      Err(e)
    } else {
      Ok(())
    }
  }
}

#[async_trait]
impl<I, M, P, G, SN, SE, SIF, SIT> Executor for KeyValueTransaction<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I>,
  SN: KeyValueStore,
  SE: KeyValueStore,
  SIF: KeyValueStore,
  SIT: KeyValueStore,
  SN::Transaction: KeyValueStoreTransaction,
  SE::Transaction: KeyValueStoreTransaction,
  SIF::Transaction: KeyValueStoreTransaction,
  SIT::Transaction: KeyValueStoreTransaction,
  GraphError: From<<SN::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SE::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIF::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIT::Transaction as KeyValueStoreExecutor>::Error>,
  <SN::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SE::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIF::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIT::Transaction as KeyValueStoreExecutor>::Error: Send,
{
  type Id = I;

  type NodeMeta = M;
  type EdgeProps = P;

  type Node = Node<I, M>;
  type Edge = Edge<I, P>;

  async fn create_node(
    &self,
    r#type: String,
    metadata: Self::NodeMeta,
  ) -> Result<Self::Node, GraphError> {
    create_node::<I, M, G, _>(&self.node_tx, &*self.id_gen, r#type, metadata).await
  }

  async fn update_node(
    &self,
    id: Self::Id,
    metadata: Self::NodeMeta,
    expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), GraphError> {
    update_node::<I, M, _>(&self.node_tx, id, metadata, expected_updated_at).await
  }

  async fn delete_node(&self, id: Self::Id) -> Result<(), GraphError> {
    delete_node::<I, P, _, _, _, _>(
      &self.node_tx,
      &self.edge_tx,
      &self.from_index_tx,
      &self.to_index_tx,
      id,
    )
    .await
  }

  async fn create_edge(
    &self,
    r#type: String,
    from_id: Self::Id,
    to_id: Self::Id,
    properties: Self::EdgeProps,
  ) -> Result<Self::Edge, GraphError> {
    create_edge::<I, P, G, _, _, _, _>(
      &self.node_tx,
      &self.edge_tx,
      &self.from_index_tx,
      &self.to_index_tx,
      &*self.id_gen,
      r#type,
      from_id,
      to_id,
      properties,
    )
    .await
  }

  async fn update_edge(
    &self,
    id: Self::Id,
    properties: Self::EdgeProps,
    expected_updated_at: Option<chrono::DateTime<chrono::Utc>>,
  ) -> Result<(), GraphError> {
    update_edge::<I, P, _>(&self.edge_tx, id, properties, expected_updated_at).await
  }

  async fn delete_edge(&self, id: Self::Id) -> Result<(), GraphError> {
    delete_edge::<I, P, _, _, _>(&self.edge_tx, &self.from_index_tx, &self.to_index_tx, id).await
  }

  async fn get_node_by_id(&self, id: Self::Id) -> Result<Option<Self::Node>, GraphError> {
    get_node_by_id::<I, M, _>(&self.node_tx, id).await
  }

  async fn get_edge_by_id(&self, id: Self::Id) -> Result<Option<Self::Edge>, GraphError> {
    get_edge_by_id::<I, P, _>(&self.edge_tx, id).await
  }

  async fn query(&self, query: Query) -> Result<Vec<Element<Self::Node, Self::Edge>>, GraphError> {
    run_query::<I, M, P, _, _, _, _>(
      &self.node_tx,
      &self.edge_tx,
      &self.from_index_tx,
      &self.to_index_tx,
      query,
    )
    .await
  }
}

#[async_trait]
impl<I, M, P, G, SN, SE, SIF, SIT> Repository for KeyValueRepository<I, M, P, G, SN, SE, SIF, SIT>
where
  I: Id,
  M: Value,
  P: Value,
  G: IdGenerator<I>,
  SN: KeyValueStore + 'static,
  SE: KeyValueStore + 'static,
  SIF: KeyValueStore + 'static,
  SIT: KeyValueStore + 'static,
  SN::Transaction: KeyValueStoreTransaction + 'static,
  SE::Transaction: KeyValueStoreTransaction + 'static,
  SIF::Transaction: KeyValueStoreTransaction + 'static,
  SIT::Transaction: KeyValueStoreTransaction + 'static,
  GraphError: From<SN::Error> + From<SE::Error> + From<SIF::Error> + From<SIT::Error>,
  SN::Error: Send,
  SE::Error: Send,
  SIF::Error: Send,
  SIT::Error: Send,
  GraphError: From<<SN::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SE::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIF::Transaction as KeyValueStoreExecutor>::Error>
    + From<<SIT::Transaction as KeyValueStoreExecutor>::Error>,
  <SN::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SE::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIF::Transaction as KeyValueStoreExecutor>::Error: Send,
  <SIT::Transaction as KeyValueStoreExecutor>::Error: Send,
{
  type Transaction = KeyValueTransaction<I, M, P, G, SN, SE, SIF, SIT>;

  async fn transaction(&self) -> Result<Self::Transaction, GraphError> {
    let node_tx = self.node_store.transaction().await?;
    let edge_tx = self.edge_store.transaction().await?;
    let from_index_tx = self.from_index_store.transaction().await?;
    let to_index_tx = self.to_index_store.transaction().await?;
    Ok(KeyValueTransaction::new(
      node_tx,
      edge_tx,
      from_index_tx,
      to_index_tx,
      self.id_gen.clone(),
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::get_json_field;

  #[cfg(feature = "in-memory")]
  use alloc::vec::Vec;

  #[cfg(feature = "in-memory")]
  use mises_async_kv_bytes::KeyValueStoreExecutor;

  #[cfg(feature = "in-memory")]
  use crate::InMemoryKeyValueStore;

  #[cfg(feature = "in-memory")]
  #[tokio::test]
  async fn scan_predicate_semantics() -> Result<(), crate::error::GraphError> {
    let store = InMemoryKeyValueStore::default();
    store.put(b"key1".as_ref(), b"v1".to_vec()).await?;
    store.put(b"key2".as_ref(), b"v2".to_vec()).await?;
    store.put(b"key3".as_ref(), b"v3".to_vec()).await?;

    let key1 = b"key1".to_vec();
    let key2 = b"key2".to_vec();
    let mut res: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    store
      .scan(
        b"key1".to_vec()..b"key9".to_vec(),
        |k: &Vec<u8>, v: &Vec<u8>| {
          if k == &key2 {
            return false;
          }
          if k == &key1 {
            res.push((k.clone(), v.clone()));
          }
          true
        },
      )
      .await?;

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].0, b"key1".to_vec());
    Ok(())
  }

  #[test]
  fn get_json_field_enum_wrapped() {
    let v = serde_json::json!({"Some": {"name": "Alice"}});
    assert_eq!(
      get_json_field(&v, "name").and_then(|x| x.as_str()),
      Some("Alice")
    );
  }
}
