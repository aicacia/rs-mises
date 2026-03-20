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
use native_authentication::{AndroidText, AuthError, Context, PolicyBuilder, Text, WindowsText};

use crate::{
  error::ToStatus,
  helpers::{OptionExt, ResultExt},
  jwt::TokenBuilder,
  oidc_service::authorization_code::{delete_authorization_code, get_authorization_code},
};

const BUILTIN_CLIENT_IDS: &[&str] = &["mises-desktop", "mises-web"];

pub async fn token<R, S>(
  repo: &R,
  device_id: &str,
  store: &S,
  req: mises_proto::TokenRequest,
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

async fn resolve_application<R>(
  repo: &R,
  device_id: &str,
  client_id_str: &str,
) -> Result<(uuid::Uuid, String), Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let app_node = if BUILTIN_CLIENT_IDS.contains(&client_id_str) {
    match identity_service
      .find_service_by_name("mises")
      .await
      .map_err(|e| Status::internal(format!("failed to find service: {}", e)))?
    {
      Some(node) => node,
      None => {
        let master_group = identity_service
          .get_master_group()
          .await
          .map_err(|e| Status::internal(format!("failed to get master group: {}", e)))?;

        let (service_node, _key_node) = identity_service
          .create_service("mises".to_owned(), Some(master_group.id))
          .await
          .map_err(|e| Status::internal(format!("failed to create service: {}", e)))?;

        service_node
      }
    }
  } else {
    let parsed_id =
      uuid::Uuid::parse_str(client_id_str).or_invalid_argument("invalid client_id format")?;

    identity_service
      .get_node_by_id_and_identity_type(parsed_id, IdentityType::Application)
      .await
      .map_err(|e| match e {
        mises_core::CoreError::NotFound => {
          Status::invalid_argument(format!("application not found: {}", parsed_id))
        }
        mises_core::CoreError::InvalidInput(_) => {
          Status::invalid_argument("client_id does not refer to an application")
        }
        _ => Status::internal(format!("identity service error: {}", e)),
      })?
  };

  let client_id = app_node.id;
  let app_id_str = match &app_node.metadata {
    NodeMeta::Identity(identity) => match identity.as_ref() {
      IdentityMeta::Application { oidc } => {
        let oidc_meta = oidc.as_ref();
        if oidc_meta.service_id.is_empty() {
          client_id.to_string()
        } else {
          oidc_meta.service_id.clone()
        }
      }
      _ => client_id.to_string(),
    },
    _ => client_id.to_string(),
  };

  Ok((client_id, app_id_str))
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
  let code = authorization_code.code.trim();
  if code.is_empty() {
    return Err(Status::invalid_argument("code is required"));
  }

  let code_data = get_authorization_code(store, code)
    .await?
    .or_invalid_argument("invalid or expired authorization code")?;

  if code_data.is_expired() {
    delete_authorization_code(store, code).await?;
    return Err(Status::invalid_argument("authorization code expired"));
  }

  if let Some(ref provided_redirect) = authorization_code.redirect_uri
    && provided_redirect != &code_data.redirect_uri
  {
    return Err(Status::invalid_argument("redirect_uri mismatch"));
  }

  if let Some(ref client_id_str) = authorization_code.client_id {
    let (resolved_client_id, _) = resolve_application(repo, device_id, client_id_str).await?;
    if resolved_client_id != code_data.client_id {
      return Err(Status::invalid_argument("client_id mismatch"));
    }
  }

  if let Some(ref code_challenge) = code_data.code_challenge {
    let verifier = authorization_code
      .code_verifier
      .as_ref()
      .or_invalid_argument("code_verifier required")?;

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

  delete_authorization_code(store, code).await?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let app_node = repo
    .get_node_by_id(code_data.client_id)
    .await
    .map_err(|e| Status::internal(format!("failed to get client: {}", e)))?
    .or_internal("client not found")?;

  let (access_token_expiry, refresh_token_expiry) = extract_client_token_expiry(&app_node)?;

  let app_key_node = identity_service
    .get_identity_key_node(app_node.id)
    .await
    .map_err(|e| e.to_status())?;

  let scope = code_data.scope.as_deref();
  let app_id_str = match &app_node.metadata {
    NodeMeta::Identity(identity) => match identity.as_ref() {
      IdentityMeta::Application { oidc } => {
        let oidc_meta = oidc.as_ref();
        if oidc_meta.service_id.is_empty() {
          code_data.client_id.to_string()
        } else {
          oidc_meta.service_id.clone()
        }
      }
      _ => code_data.client_id.to_string(),
    },
    _ => code_data.client_id.to_string(),
  };
  let subject_str = code_data.subject.to_string();

  let access_token = TokenBuilder::new(&app_key_node)
    .sub(&app_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .acting_for(&subject_str)
    .expires_in(access_token_expiry)
    .token_type("access")
    .build()?;

  let refresh_token = TokenBuilder::new(&app_key_node)
    .sub(&app_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .acting_for(&subject_str)
    .expires_in(refresh_token_expiry)
    .token_type("refresh")
    .build()?;

  let id_token = if scope.is_some_and(|s| s.contains("openid")) {
    Some(
      TokenBuilder::new(&app_key_node)
        .sub(&subject_str)
        .issuer(issuer)
        .audience(&app_id_str)
        .nonce(code_data.nonce.as_deref().unwrap_or(""))
        .expires_in(access_token_expiry)
        .token_type("id")
        .build()?,
    )
  } else {
    None
  };

  log::debug!(
    "Issued tokens for client_id: {}, subject: {}, scope: {:?}",
    app_id_str,
    subject_str,
    code_data.scope
  );

  Ok(mises_proto::TokenResponse {
    expires_in: Some(access_token_expiry as u64),
    access_token,
    token_type: "Bearer".to_string(),
    refresh_token_expires_in: Some(refresh_token_expiry as u64),
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

  let scope = grant.scope.as_deref();
  if !scope.is_some_and(|s| s.contains("openid")) {
    return Err(Status::invalid_argument("scope must include 'openid'"));
  }

  if grant.client_id.is_empty() {
    return Err(Status::invalid_argument("client_id is required"));
  }

  let (client_id, app_id_str) = resolve_application(repo, device_id, &grant.client_id).await?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());
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

  let app_node = repo
    .get_node_by_id(client_id)
    .await
    .map_err(|e| Status::internal(format!("failed to get client: {}", e)))?
    .or_internal("client not found")?;
  let (access_token_expiry, refresh_token_expiry) = extract_client_token_expiry(&app_node)?;

  let access_token = TokenBuilder::new(&user_key_node)
    .sub(&user_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .expires_in(access_token_expiry)
    .token_type("access")
    .build()?;

  let refresh_token = TokenBuilder::new(&user_key_node)
    .sub(&user_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .expires_in(refresh_token_expiry)
    .token_type("refresh")
    .build()?;

  let id_token = TokenBuilder::new(&user_key_node)
    .sub(&user_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .expires_in(access_token_expiry)
    .token_type("id")
    .build()?;

  Ok(mises_proto::TokenResponse {
    expires_in: Some(access_token_expiry as u64),
    access_token,
    token_type: "Bearer".to_string(),
    refresh_token_expires_in: Some(refresh_token_expiry as u64),
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
  if grant.client_id.is_empty() {
    return Err(Status::invalid_argument("client_id is required"));
  }

  let (_client_id, app_id_str) = resolve_application(repo, device_id, &grant.client_id).await?;

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

  let access_token = TokenBuilder::new(&device_key_node)
    .sub(&device_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .expires_in(access_token_expiry)
    .token_type("access")
    .build()?;

  let refresh_token = TokenBuilder::new(&device_key_node)
    .sub(&device_id_str)
    .issuer(issuer)
    .audience(&app_id_str)
    .scope(scope.unwrap_or("").to_string())
    .expires_in(refresh_token_expiry)
    .token_type("refresh")
    .build()?;

  let id_token = if scope.is_some_and(|s| s.contains("openid")) {
    Some(
      TokenBuilder::new(&device_key_node)
        .sub(&device_id_str)
        .issuer(issuer)
        .audience(&app_id_str)
        .expires_in(access_token_expiry)
        .token_type("id")
        .build()?,
    )
  } else {
    None
  };

  Ok(mises_proto::TokenResponse {
    expires_in: Some(access_token_expiry as u64),
    access_token,
    token_type: "Bearer".to_string(),
    refresh_token_expires_in: Some(refresh_token_expiry as u64),
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
      IdentityMeta::Application { oidc } => {
        let oidc_meta = oidc.as_ref();
        let access_expiry = oidc_meta.access_token_expiry as i64;
        let refresh_expiry = oidc_meta.refresh_token_expiry as i64;
        Ok((access_expiry, refresh_expiry))
      }
      IdentityMeta::Service { .. } => Ok((3600, 2592000)),
      _ => Err(Status::internal(
        "client node is not an application or service",
      )),
    },
    _ => Err(Status::internal("client node is not an identity")),
  }
}
