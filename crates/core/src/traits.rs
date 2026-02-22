use uuid::Uuid;

use mises_graph::{
  Edge as MisesGraphEdge, Executor as MisesGraphExecutor, Node as MisesGraphNode,
  Repository as MisesGraphRepository,
};

use crate::model::{edge::EdgeProps, node::NodeMeta};

/// Core node type with UUID identifiers and node metadata.
pub type Node = MisesGraphNode<Uuid, NodeMeta>;

/// Core edge type with UUID identifiers and edge properties.
pub type Edge = MisesGraphEdge<Uuid, EdgeProps>;

/// Graph repository trait for core operations.
///
/// Implements the underlying graph repository with UUID identifiers and core metadata types.
pub trait Repository:
  MisesGraphRepository<
    Id = Uuid,
    NodeMeta = NodeMeta,
    EdgeProps = EdgeProps,
    Node = Node,
    Edge = Edge,
  >
{
}

impl<T> Repository for T where
  T: MisesGraphRepository<
      Id = Uuid,
      NodeMeta = NodeMeta,
      EdgeProps = EdgeProps,
      Node = Node,
      Edge = Edge,
    >
{
}

/// Graph executor trait for core transactional operations.
///
/// Implements the underlying graph executor with UUID identifiers and core metadata types.
pub trait Executor:
  MisesGraphExecutor<Id = Uuid, NodeMeta = NodeMeta, EdgeProps = EdgeProps, Node = Node, Edge = Edge>
{
}

impl<T> Executor for T where
  T: MisesGraphExecutor<
      Id = Uuid,
      NodeMeta = NodeMeta,
      EdgeProps = EdgeProps,
      Node = Node,
      Edge = Edge,
    >
{
}
