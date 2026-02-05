use mises_graph::{ComparisonOp, EdgeQuery, NodeQuery, Query, field};

#[test]
fn build_node_and_edge_queries() {
  // Basic node query
  let q = NodeQuery::new("User")
    .filter(
      field("metadata.age")
        .gte(18)
        .and(field("metadata.country").eq("US")),
    )
    .limit(10);

  // Edge query with nested node filters
  let eq = EdgeQuery::outgoing("HAS_KEY")
    .filter(field("properties.scope").eq("owner"))
    .to(NodeQuery::new("Key"));

  // Node query that includes edges to load
  let node_with_edges = NodeQuery::new("User").include(eq.clone());

  // Combined query to run both node and edge queries in one call
  let _combined = Query::nodes(node_with_edges.clone()).with_edge(eq.clone());

  // Basic assertions that things built as expected
  assert_eq!(q.node_type.as_deref(), Some("User"));
  assert_eq!(eq.edge_type.as_deref(), Some("HAS_KEY"));
  assert_eq!(node_with_edges.node_type.as_deref(), Some("User"));
}

#[test]
fn field_exists_predicate() {
  let p = field("metadata.derivation_path").exists();
  assert_eq!(p.field, "metadata.derivation_path");
  assert_eq!(p.op, ComparisonOp::Exists);
  assert_eq!(p.value, None);
}
