use uuid::Uuid;

use crate::{
  CoreError, Result,
  model::{identity::IdentityType, node::NodeMeta},
  traits::Repository,
};

pub struct IdentityService<R>
where
  R: Repository,
{
  repo: R,
}

impl<R> IdentityService<R>
where
  R: Repository,
{
  pub async fn get_identity_type(&self, id: Uuid) -> Result<IdentityType> {
    let node = self
      .repo
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match &node.metadata {
      NodeMeta::Identity(identity_meta) => Ok(identity_meta.identity_type()),
      _ => Err(CoreError::InvalidInput(crate::error::InvalidInput::Other(
        "node is not an identity".into(),
      ))),
    }
  }
}
