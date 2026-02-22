use tonic::Status;
use uuid::Uuid;

use mises_core::{
  model::identity::{IdentityMeta, IdentityType},
  service::identity::IdentityService,
  traits::Repository,
};

use crate::{
  error::ToStatus,
  helpers::{OptionExt, ResultExt},
  jwt::Claims,
};

pub async fn get_user_info<R>(
  repo: &R,
  device_id: &str,
  claims: Option<Claims>,
) -> Result<mises_proto::UserInfo, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let claims = claims.or_unauthenticated("authorization required: bearer token not provided")?;

  let user_id = Uuid::parse_str(&claims.sub).or_internal("invalid subject in token")?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let user_node = identity_service
    .get_node_by_id_and_identity_type(user_id, IdentityType::User)
    .await
    .map_err(|e| match e {
      mises_core::CoreError::NotFound => Status::not_found("user not found"),
      _ => e.to_status(),
    })?;

  let (name, given_name, family_name, preferred_username, email) = match &user_node.metadata {
    mises_core::model::node::NodeMeta::Identity(identity) => match identity.as_ref() {
      IdentityMeta::User { name, .. } => (Some(name.clone()), None, None, Some(name.clone()), None),
      _ => (None, None, None, None, None),
    },
    _ => (None, None, None, None, None),
  };

  Ok(mises_proto::UserInfo {
    sub: user_id.to_string(),
    permissions: vec![],
    name,
    given_name,
    family_name,
    preferred_username,
    email,
    email_verified: None,
    picture: None,
  })
}
