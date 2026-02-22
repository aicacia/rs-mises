#![cfg(feature = "in-memory")]

use mises_graph::{EdgeQuery, Element, Executor, NodeQuery, Query, field};
use serde_json::json;

mod common;

use common::U64Generator;

#[tokio::test]
async fn edge_query_filters_by_node_id() {
  use mises_graph::InMemoryKeyValueRepository;
  type Repo = InMemoryKeyValueRepository<u64, serde_json::Value, serde_json::Value, U64Generator>;
  let repo = Repo::new_in_memory(U64Generator::new_u64());

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
  let _other_edge = repo
    .create_edge("TEST_EDGE".to_string(), n2.id, n1.id, json!({ "at": 2 }))
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
  let edges: Vec<_> = elements
    .into_iter()
    .filter_map(|el| match el {
      Element::Edge(edge) => Some(edge),
      _ => None,
    })
    .collect();
  assert_eq!(edges.len(), 1, "expected exactly one edge after filtering");
  assert_eq!(edges[0].from_id, n1.id);

  let negative = Query::edges(
    EdgeQuery::outgoing("TEST_EDGE").from(NodeQuery::any().filter(field("id").eq(999_u64))),
  );
  let negative_elements = repo.query(negative).await.unwrap();
  assert!(
    negative_elements.is_empty(),
    "expected no edges for unknown node id"
  );
}
