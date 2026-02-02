use core::ops::Not;

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Query options such as `limit` for pagination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct QueryOptions {
  pub limit: Option<usize>,
}

impl QueryOptions {
  pub fn new() -> Self {
    Self::default()
  }

  #[must_use]
  pub fn limit(mut self, limit: usize) -> Self {
    self.limit = Some(limit);
    self
  }
}

/// Comparison operators for predicates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOp {
  Eq,
  Ne,
  Gt,
  Gte,
  Lt,
  Lte,
  Exists,
  In,
  Contains,
}

/// A simple predicate: `field op value`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Predicate {
  pub field: String,
  pub op: ComparisonOp,
  pub value: Option<JsonValue>,
}

impl Predicate {
  pub fn and(self, other: impl Into<Filter>) -> Filter {
    Filter::And(vec![Filter::Predicate(self), other.into()])
  }

  pub fn or(self, other: impl Into<Filter>) -> Filter {
    Filter::Or(vec![Filter::Predicate(self), other.into()])
  }

  pub fn into_filter(self) -> Filter {
    Filter::Predicate(self)
  }
}

impl Not for Predicate {
  type Output = Filter;

  fn not(self) -> Self::Output {
    Filter::Not(Box::new(Filter::Predicate(self)))
  }
}

/// Logical filter tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Filter {
  Predicate(Predicate),
  And(Vec<Filter>),
  Or(Vec<Filter>),
  Not(Box<Filter>),
}

impl From<Predicate> for Filter {
  fn from(p: Predicate) -> Self {
    Filter::Predicate(p)
  }
}

impl Filter {
  pub fn all<I>(filters: I) -> Self
  where
    I: IntoIterator<Item = Filter>,
  {
    Filter::And(filters.into_iter().collect())
  }

  pub fn any<I>(filters: I) -> Self
  where
    I: IntoIterator<Item = Filter>,
  {
    Filter::Or(filters.into_iter().collect())
  }

  pub fn and(self, other: Filter) -> Self {
    match self {
      Filter::And(mut v) => {
        v.push(other);
        Filter::And(v)
      }
      _ => Filter::And(vec![self, other]),
    }
  }

  pub fn or(self, other: Filter) -> Self {
    match self {
      Filter::Or(mut v) => {
        v.push(other);
        Filter::Or(v)
      }
      _ => Filter::Or(vec![self, other]),
    }
  }
}

impl Not for Filter {
  type Output = Self;

  fn not(self) -> Self::Output {
    Filter::Not(Box::new(self))
  }
}

/// Helper to build a field access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field(pub String);

pub fn field<S: Into<String>>(s: S) -> Field {
  Field(s.into())
}

impl Field {
  pub fn eq<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Eq,
      value: Some(v.into()),
    }
  }

  pub fn ne<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Ne,
      value: Some(v.into()),
    }
  }

  pub fn gt<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Gt,
      value: Some(v.into()),
    }
  }

  pub fn gte<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Gte,
      value: Some(v.into()),
    }
  }

  pub fn lt<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Lt,
      value: Some(v.into()),
    }
  }

  pub fn lte<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Lte,
      value: Some(v.into()),
    }
  }

  pub fn exists(self) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Exists,
      value: None,
    }
  }

  pub fn missing(self) -> Filter {
    !self.exists()
  }

  pub fn one_of<I, V>(self, values: I) -> Predicate
  where
    I: IntoIterator<Item = V>,
    V: Into<JsonValue>,
  {
    Predicate {
      field: self.0,
      op: ComparisonOp::In,
      value: Some(JsonValue::Array(
        values.into_iter().map(Into::into).collect(),
      )),
    }
  }

  pub fn contains<V: Into<JsonValue>>(self, v: V) -> Predicate {
    Predicate {
      field: self.0,
      op: ComparisonOp::Contains,
      value: Some(v.into()),
    }
  }
}

/// Direction for edge traversal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeDirection {
  Out,
  In,
  Both,
}

/// Node query AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeQuery {
  pub node_type: Option<String>,
  pub filter: Option<Filter>,
  pub include_edges: Vec<EdgeQuery>,
  pub options: QueryOptions,
}

impl NodeQuery {
  pub fn new<S: Into<String>>(t: S) -> Self {
    Self {
      node_type: Some(t.into()),
      filter: None,
      include_edges: Vec::new(),
      options: QueryOptions::default(),
    }
  }

  pub fn any() -> Self {
    Self {
      node_type: None,
      filter: None,
      include_edges: Vec::new(),
      options: QueryOptions::default(),
    }
  }

  #[must_use]
  pub fn filter(mut self, filter: impl Into<Filter>) -> Self {
    self.filter = Some(filter.into());
    self
  }

  #[must_use]
  pub fn include(mut self, eq: EdgeQuery) -> Self {
    self.include_edges.push(eq);
    self
  }

  #[must_use]
  pub fn limit(mut self, limit: usize) -> Self {
    self.options.limit = Some(limit);
    self
  }

  #[must_use]
  pub fn options(mut self, options: QueryOptions) -> Self {
    self.options = options;
    self
  }
}

/// Edge query AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeQuery {
  pub edge_type: Option<String>,
  pub direction: EdgeDirection,
  pub filter: Option<Filter>,
  pub from: Option<Box<NodeQuery>>,
  pub to: Option<Box<NodeQuery>>,
  pub options: QueryOptions,
}

impl EdgeQuery {
  pub fn new<S: Into<String>>(t: S) -> Self {
    Self {
      edge_type: Some(t.into()),
      direction: EdgeDirection::Both,
      filter: None,
      from: None,
      to: None,
      options: QueryOptions::default(),
    }
  }

  pub fn any() -> Self {
    Self {
      edge_type: None,
      direction: EdgeDirection::Both,
      filter: None,
      from: None,
      to: None,
      options: QueryOptions::default(),
    }
  }

  pub fn outgoing<S: Into<String>>(t: S) -> Self {
    Self::new(t).direction(EdgeDirection::Out)
  }

  pub fn incoming<S: Into<String>>(t: S) -> Self {
    Self::new(t).direction(EdgeDirection::In)
  }

  #[must_use]
  pub fn direction(mut self, d: EdgeDirection) -> Self {
    self.direction = d;
    self
  }

  #[must_use]
  pub fn filter(mut self, filter: impl Into<Filter>) -> Self {
    self.filter = Some(filter.into());
    self
  }

  #[must_use]
  pub fn from(mut self, q: NodeQuery) -> Self {
    self.from = Some(Box::new(q));
    self
  }

  #[must_use]
  pub fn to(mut self, q: NodeQuery) -> Self {
    self.to = Some(Box::new(q));
    self
  }

  #[must_use]
  pub fn limit(mut self, limit: usize) -> Self {
    self.options.limit = Some(limit);
    self
  }

  #[must_use]
  pub fn options(mut self, options: QueryOptions) -> Self {
    self.options = options;
    self
  }
}

/// Combined query that can carry node and/or edge sub-queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Query {
  pub node: Option<NodeQuery>,
  pub edge: Option<EdgeQuery>,
  pub options: QueryOptions,
}

impl Default for Query {
  fn default() -> Self {
    Self::new()
  }
}

impl Query {
  pub fn new() -> Self {
    Self {
      node: None,
      edge: None,
      options: QueryOptions::default(),
    }
  }

  pub fn nodes(node: NodeQuery) -> Self {
    Self::new().with_node(node)
  }

  pub fn edges(edge: EdgeQuery) -> Self {
    Self::new().with_edge(edge)
  }

  #[must_use]
  pub fn with_node(mut self, node: NodeQuery) -> Self {
    self.node = Some(node);
    self
  }

  #[must_use]
  pub fn with_edge(mut self, edge: EdgeQuery) -> Self {
    self.edge = Some(edge);
    self
  }

  #[must_use]
  pub fn limit(mut self, limit: usize) -> Self {
    self.options.limit = Some(limit);
    self
  }

  #[must_use]
  pub fn options(mut self, options: QueryOptions) -> Self {
    self.options = options;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
}
