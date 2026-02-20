use native_authentication::{AndroidText, AuthError, Context, PolicyBuilder, Text, WindowsText};
use sha2::{Digest, Sha256};
use tonic::Status;

use mises_core::{
  model::{
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
  },
  service::identity::IdentityService,
  traits::Repository,
};
use mises_graph::KeyValueStoreExecutor;

use crate::{
  error::ToStatus,
  jwt::{Claims, generate_access_token, generate_id_token, generate_refresh_token},
  oidc_service::{
    authorization_code::get_and_delete_authorization_code, helpers::ensure_application_identity,
  },
};

pub async fn token<R, S>(
  repo: &R,
  device_id: &str,
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
      handle_password_grant(repo, device_id, password_grant, issuer).await
    }
    Some(mises_proto::token_request::Grant::AuthorizationCode(authorization_code)) => {
      handle_authorization_code_grant(repo, device_id, store, authorization_code, issuer).await
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
    Some(mises_proto::token_request::Grant::DeviceCredentials(device_credentials)) => {
      handle_device_credentials_grant(repo, device_id, device_credentials, issuer).await
    }
    None => Err(Status::invalid_argument("grant type is required")),
  }
}

async fn handle_authorization_code_grant<R, S>(
  repo: &R,
  device_id: &str,
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

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let service_id = resolve_service_id_for_client(repo, device_id, code_data.client_id).await?;

  let client_node = ensure_application_identity(&identity_service, code_data.client_id).await?;

  let (access_token_expiry, refresh_token_expiry) = extract_client_token_expiry(&client_node)?;

  let client_key_node = identity_service
    .get_identity_key_node(code_data.client_id)
    .await
    .map_err(|e| e.to_status())?;

  let scope = code_data.scope.as_deref();
  let client_id_str = code_data.client_id.to_string();
  let subject_str = code_data.subject.to_string();

  let access_token = generate_access_token(
    &client_key_node,
    &client_id_str,
    issuer,
    &service_id,
    scope,
    Some(&subject_str),
    access_token_expiry,
  )?;

  let refresh_token = generate_refresh_token(
    &client_key_node,
    &client_id_str,
    issuer,
    &service_id,
    scope,
    Some(&subject_str),
    refresh_token_expiry,
  )?;

  let id_token = if scope.is_some_and(|s| s.contains("openid")) {
    Some(generate_id_token(
      &client_key_node,
      &subject_str,
      issuer,
      &service_id,
      code_data.nonce.as_deref(),
      access_token_expiry,
    )?)
  } else {
    None
  };

  Ok(mises_proto::TokenResponse {
    access_token,
    token_type: "Bearer".to_string(),
    expires_in: Some(access_token_expiry as u64),
    refresh_token: Some(refresh_token),
    id_token,
    scope: code_data.scope,
  })
}

async fn handle_password_grant<R>(
  repo: &R,
  device_id: &str,
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

  let scope = grant.scope.as_deref();
  if !scope.is_some_and(|s| s.contains("openid")) {
    return Err(Status::invalid_argument("scope must include 'openid'"));
  }

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let service_id = resolve_default_service_id(repo, device_id).await?;
  let user_node = identity_service
    .authenticate_user(&grant.username, &grant.password)
    .await
    .map_err(|e| match e {
      mises_core::CoreError::NotFound => Status::unauthenticated("invalid username or password"),
      other => other.to_status(),
    })?;

  let user_id = user_node.id;

  let user_key_node = identity_service
    .get_identity_key_node(user_id)
    .await
    .map_err(|e| e.to_status())?;

  let user_id_str = user_id.to_string();

  let access_token = generate_access_token(
    &user_key_node,
    &user_id_str,
    issuer,
    &service_id,
    scope,
    None,
    access_token_expiry,
  )?;

  let refresh_token = generate_refresh_token(
    &user_key_node,
    &user_id_str,
    issuer,
    &service_id,
    scope,
    None,
    refresh_token_expiry,
  )?;

  let id_token = generate_id_token(
    &user_key_node,
    &user_id_str,
    issuer,
    &service_id,
    None,
    access_token_expiry,
  )?;

  Ok(mises_proto::TokenResponse {
    access_token,
    token_type: "Bearer".to_string(),
    expires_in: Some(access_token_expiry as u64),
    refresh_token: Some(refresh_token),
    id_token: Some(id_token),
    scope: grant.scope,
  })
}

async fn handle_device_credentials_grant<R>(
  repo: &R,
  device_id: &str,
  grant: mises_proto::DeviceCredentials,
  issuer: &str,
) -> Result<mises_proto::TokenResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let policy = PolicyBuilder::new()
    .password(true)
    .build()
    .map_err(|_| Status::internal("device auth policy error"))?;

  let text = Text {
    android: AndroidText {
      title: "Mises device access".to_string(),
      subtitle: None,
      description: None,
    },
    apple: "Mises device access".to_string(),
    windows: WindowsText::new("Mises device access", "Allow Mises to access this device"),
  };

  let ctx = Context::new(());
  ctx.authenticate(text, &policy).await.map_err(|e| match e {
    AuthError::NotSupported => Status::failed_precondition("native authentication not supported"),
    AuthError::MissingTool => Status::failed_precondition("pkcheck not available"),
    AuthError::ExecutionError(_) => Status::permission_denied("device authentication failed"),
  })?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let service_id = resolve_default_service_id(repo, device_id).await?;
  let master_group = identity_service
    .get_master_group()
    .await
    .map_err(|e| e.to_status())?;

  let device_node = identity_service
    .find_this_device(master_group.id)
    .await
    .map_err(|e| e.to_status())?
    .ok_or_else(|| Status::failed_precondition("no device identity found"))?;

  let device_key_node = identity_service
    .get_identity_key_node(device_node.id)
    .await
    .map_err(|e| e.to_status())?;

  let access_token_expiry = 900;
  let refresh_token_expiry = 604800;
  let scope = grant.scope.as_deref();
  let device_id_str = device_node.id.to_string();

  let access_token = generate_access_token(
    &device_key_node,
    &device_id_str,
    issuer,
    &service_id,
    scope,
    None,
    access_token_expiry,
  )?;

  let refresh_token = generate_refresh_token(
    &device_key_node,
    &device_id_str,
    issuer,
    &service_id,
    scope,
    None,
    refresh_token_expiry,
  )?;

  let id_token = if scope.is_some_and(|s| s.contains("openid")) {
    Some(generate_id_token(
      &device_key_node,
      &device_id_str,
      issuer,
      &service_id,
      None,
      access_token_expiry,
    )?)
  } else {
    None
  };

  Ok(mises_proto::TokenResponse {
    access_token,
    token_type: "Bearer".to_string(),
    expires_in: Some(access_token_expiry as u64),
    refresh_token: Some(refresh_token),
    id_token,
    scope: grant.scope,
  })
}

fn extract_client_token_expiry(
  client_node: &mises_core::traits::Node,
) -> Result<(i64, i64), Status> {
  match &client_node.metadata {
    NodeMeta::Identity(identity) => match identity.as_ref() {
      IdentityMeta::Application { oidc, .. } => match oidc.as_ref() {
        Some(oidc_meta) => {
          let access_expiry = oidc_meta.access_token_expiry as i64;
          let refresh_expiry = oidc_meta.refresh_token_expiry as i64;
          Ok((access_expiry, refresh_expiry))
        }
        None => Err(Status::internal(
          "client node application has no OIDC metadata",
        )),
      },
      _ => Err(Status::internal("client node is not an application")),
    },
    _ => Err(Status::internal("client node is not an identity")),
  }
}

async fn resolve_default_service_id<R>(repo: &R, device_id: &str) -> Result<String, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let service = identity_service
    .find_service_by_name("mises")
    .await
    .map_err(|e| Status::internal(format!("failed to find mises service: {}", e)))?
    .ok_or_else(|| Status::not_found("mises service not found"))?;

  Ok(service.id.to_string())
}

async fn resolve_service_id_for_client<R>(
  repo: &R,
  device_id: &str,
  client_id: uuid::Uuid,
) -> Result<String, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
  let service = identity_service
    .find_owner(client_id, Some(IdentityType::Service))
    .await
    .map_err(|e| Status::internal(format!("failed to find client owner: {}", e)))?
    .ok_or_else(|| Status::not_found("client owner service not found"))?;

  Ok(service.id.to_string())
}
