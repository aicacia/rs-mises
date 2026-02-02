use mises_graph::{
  Edge as MisesGraphEdge, Node as MisesGraphNode, Repository as MisesGraphRepository,
};
use uuid::Uuid;

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
