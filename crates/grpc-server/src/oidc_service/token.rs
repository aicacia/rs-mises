use sha2::{Digest, Sha256};
use tonic::Status;

use mises_core::{
  model::{identity::IdentityMeta, node::NodeMeta},
  service::identity::IdentityService,
  traits::Repository,
};
use mises_graph::KeyValueStoreExecutor;

use crate::{
  error::ToStatus,
  jwt::{Claims, generate_access_token, generate_refresh_token},
  oidc_service::{
    authorization_code::get_and_delete_authorization_code, helpers::ensure_application_identity,
  },
};

pub async fn token<R, S>(
  repo: &R,
  store: &S,
  req: mises_proto::TokenRequest,
  _claims: Option<Claims>,
  issuer: &str,
) -> Result<mises_proto::TokenResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
  S: KeyValueStoreExecutor,
{
  match req.grant {
    Some(mises_proto::token_request::Grant::Password(password_grant)) => {
      handle_password_grant(repo, password_grant, issuer).await
    }
    Some(mises_proto::token_request::Grant::AuthorizationCode(authorization_code)) => {
      handle_authorization_code_grant(repo, store, authorization_code, issuer).await
    }
    Some(mises_proto::token_request::Grant::RefreshToken(_)) => {
      Err(Status::unimplemented("refresh_token grant not implemented"))
    }
    Some(mises_proto::token_request::Grant::ClientCredentials(_)) => Err(Status::unimplemented(
      "client_credentials grant not implemented",
    )),
    Some(mises_proto::token_request::Grant::DeviceCode(_)) => {
      Err(Status::unimplemented("device_code grant not implemented"))
    }
    None => Err(Status::invalid_argument("grant type is required")),
  }
}

async fn handle_authorization_code_grant<R, S>(
  repo: &R,
  store: &S,
  authorization_code: mises_proto::AuthorizationCode,
  issuer: &str,
) -> Result<mises_proto::TokenResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
  S: KeyValueStoreExecutor,
{
  let code_data = get_and_delete_authorization_code(store, &authorization_code.code)
    .await?
    .ok_or_else(|| Status::invalid_argument("invalid or expired authorization code"))?;

  if code_data.is_expired() {
    return Err(Status::invalid_argument("authorization code expired"));
  }

  if let Some(ref provided_redirect) = authorization_code.redirect_uri
    && provided_redirect != &code_data.redirect_uri
  {
    return Err(Status::invalid_argument("redirect_uri mismatch"));
  }

  if let Some(ref client_id_str) = authorization_code.client_id {
    let provided_client = uuid::Uuid::parse_str(client_id_str)
      .map_err(|_| Status::invalid_argument("invalid client_id"))?;
    if provided_client != code_data.client_id {
      return Err(Status::invalid_argument("client_id mismatch"));
    }
  }

  if let Some(ref code_challenge) = code_data.code_challenge {
    let verifier = authorization_code
      .code_verifier
      .as_ref()
      .ok_or_else(|| Status::invalid_argument("code_verifier required"))?;

    let method = code_data
      .code_challenge_method
      .as_deref()
      .unwrap_or("plain");
    let computed_challenge = match method {
      "S256" => {
        let hash = Sha256::digest(verifier.as_bytes());
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, hash)
      }
      "plain" => verifier.clone(),
      _ => {
        return Err(Status::invalid_argument(
          "unsupported code_challenge_method",
        ));
      }
    };

    if &computed_challenge != code_challenge {
      return Err(Status::invalid_argument(
        "code_verifier does not match code_challenge",
      ));
    }
  }

  let client_node = ensure_application_identity(repo, code_data.client_id).await?;

  let (access_token_expiry, refresh_token_expiry) = extract_client_token_expiry(&client_node)?;

  let identity_service = IdentityService::new(repo.clone());
  let (_key_node, client_key) = identity_service
    .get_identity_key(code_data.client_id)
    .await
    .map_err(|e| e.to_status())?;

  let scope = code_data.scope.as_deref();

  let access_token = generate_access_token(
    &code_data.subject.to_string(),
    issuer,
    scope,
    access_token_expiry,
    &client_key,
  )?;

  let refresh_token = generate_refresh_token(
    &code_data.subject.to_string(),
    issuer,
    scope,
    refresh_token_expiry,
    &client_key,
  )?;

  Ok(mises_proto::TokenResponse {
    access_token,
    token_type: "Bearer".to_string(),
    expires_in: Some(access_token_expiry as u64),
    refresh_token: Some(refresh_token),
    id_token: None,
    scope: code_data.scope,
  })
}

async fn handle_password_grant<R>(
  repo: &R,
  grant: mises_proto::Password,
  issuer: &str,
) -> Result<mises_proto::TokenResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  if grant.username.is_empty() {
    return Err(Status::invalid_argument("username is required"));
  }

  if grant.password.is_empty() {
    return Err(Status::invalid_argument("password is required"));
  }

  let access_token_expiry = 900;
  let refresh_token_expiry = 604800;

  let identity_service = IdentityService::new(repo.clone());
  let user_node = identity_service
    .authenticate_user(&grant.username, &grant.password)
    .await
    .map_err(|e| match e {
      mises_core::CoreError::NotFound => Status::unauthenticated("invalid username or password"),
      other => other.to_status(),
    })?;

  let user_id = user_node.id;

  let (_key_node, user_key) = identity_service
    .get_identity_key(user_id)
    .await
    .map_err(|e| e.to_status())?;

  let scope = grant.scope.as_deref();

  let access_token = generate_access_token(
    &user_id.to_string(),
    issuer,
    scope,
    access_token_expiry,
    &user_key,
  )?;

  let refresh_token = generate_refresh_token(
    &user_id.to_string(),
    issuer,
    scope,
    refresh_token_expiry,
    &user_key,
  )?;

  Ok(mises_proto::TokenResponse {
    access_token,
    token_type: "Bearer".to_string(),
    expires_in: Some(access_token_expiry as u64),
    refresh_token: Some(refresh_token),
    id_token: None,
    scope: grant.scope,
  })
}

fn extract_client_token_expiry(
  client_node: &mises_core::traits::Node,
) -> Result<(i64, i64), Status> {
  match &client_node.metadata {
    NodeMeta::Identity(IdentityMeta::Application {
      oidc: Some(oidc_meta),
      ..
    }) => {
      let access_expiry = oidc_meta.access_token_expiry as i64;
      let refresh_expiry = oidc_meta.refresh_token_expiry as i64;
      Ok((access_expiry, refresh_expiry))
    }
    _ => Err(Status::internal(
      "client node is not an application with OIDC metadata",
    )),
  }
}
