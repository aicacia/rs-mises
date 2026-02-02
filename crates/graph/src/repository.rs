use alloc::{boxed::Box, string::String, vec::Vec};

use chrono::{DateTime, Utc};

use crate::{
  error::GraphError,
  query::Query,
  types::{Element, Id, Value},
};

#[async_trait::async_trait]
pub trait Executor: Send + Sync {
  type Id: Id;

  type NodeMeta: Value;
  type EdgeProps: Value;

  type Node: Value;
  type Edge: Value;

  async fn get_node_by_id(&self, id: Self::Id) -> Result<Option<Self::Node>, GraphError>;
  async fn create_node(
    &self,
    r#type: String,
    metadata: Self::NodeMeta,
  ) -> Result<Self::Node, GraphError>;
  async fn update_node(
    &self,
    id: Self::Id,
    metadata: Self::NodeMeta,
    expected_updated_at: Option<DateTime<Utc>>,
  ) -> Result<(), GraphError>;
  async fn delete_node(&self, id: Self::Id) -> Result<(), GraphError>;

  async fn get_edge_by_id(&self, id: Self::Id) -> Result<Option<Self::Edge>, GraphError>;
  async fn create_edge(
    &self,
    r#type: String,
    from_id: Self::Id,
    to_id: Self::Id,
    properties: Self::EdgeProps,
  ) -> Result<Self::Edge, GraphError>;
  async fn update_edge(
    &self,
    id: Self::Id,
    properties: Self::EdgeProps,
    expected_updated_at: Option<DateTime<Utc>>,
  ) -> Result<(), GraphError>;
  async fn delete_edge(&self, id: Self::Id) -> Result<(), GraphError>;

  async fn query(&self, query: Query) -> Result<Vec<Element<Self::Node, Self::Edge>>, GraphError>;
}

#[async_trait::async_trait]
pub trait Transaction: Executor + Sized {
  async fn commit(self) -> Result<(), GraphError>;
  async fn rollback(self) -> Result<(), GraphError>;
}

#[async_trait::async_trait]
pub trait Repository: Executor {
  type Transaction: Transaction<
      Id = Self::Id,
      NodeMeta = Self::NodeMeta,
      EdgeProps = Self::EdgeProps,
      Node = Self::Node,
      Edge = Self::Edge,
    >;

  async fn transaction(&self) -> Result<Self::Transaction, GraphError>;
}
