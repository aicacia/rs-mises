use core::hash::Hash;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait Id:
  Serialize + DeserializeOwned + Clone + PartialEq + Eq + Ord + Hash + Send + Sync + 'static
{
}

impl<T> Id for T where
  T: Serialize + DeserializeOwned + Clone + PartialEq + Eq + Ord + Hash + Send + Sync + 'static
{
}

pub trait Value:
  Serialize + DeserializeOwned + Clone + Send + Sync + 'static + PartialEq + Eq
{
}

impl<T> Value for T where
  T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static + PartialEq + Eq
{
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Element<N, E> {
  Node(N),
  Edge(E),
}
