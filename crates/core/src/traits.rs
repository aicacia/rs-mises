use uuid::Uuid;

use mises_graph::{
  Edge as MisesGraphEdge, Executor as MisesGraphExecutor, Node as MisesGraphNode,
  Repository as MisesGraphRepository,
};

use crate::model::{edge::EdgeProps, node::NodeMeta};

pub type Node = MisesGraphNode<Uuid, NodeMeta>;
pub type Edge = MisesGraphEdge<Uuid, EdgeProps>;

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
