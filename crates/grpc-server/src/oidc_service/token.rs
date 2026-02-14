use mises_graph::{EdgeQuery, KeyValueStoreExecutor};
use tonic::Status;

use mises_core::{
  model::{
    edge::EdgeType,
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
  },
  service::password,
  traits::Repository,
};
use mises_graph::{Element, Filter, NodeQuery, Query, field};

use crate::jwt::{Claims, generate_access_token, generate_refresh_token};
use crate::oidc_service::{
  authorization_code::get_and_delete_authorization_code, helpers::ensure_application_identity,
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
        use sha2::{Digest, Sha256};
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

  let client_key = get_signing_key(repo, code_data.client_id).await?;

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

  // let client_uuid = uuid::Uuid::parse_str(&grant.client_id)
  //   .map_err(|_| Status::invalid_argument("invalid client_id"))?;

  // let client_node = ensure_application_identity(repo, client_uuid).await?;

  // let (access_token_expiry, refresh_token_expiry) = extract_client_token_expiry(&client_node)?;

  let access_token_expiry = 900; // 15 minutes in seconds
  let refresh_token_expiry = 604800; // 7 days in seconds

  let user_node = find_user_by_name(repo, &grant.username, &grant.password).await?;

  let user_id = user_node.id;

  let user_key = get_signing_key(repo, user_id).await?;

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

async fn find_user_by_name<R>(
  repo: &R,
  username: &str,
  password_plain: &str,
) -> Result<R::Node, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  use mises_core::model::node::NodeType;

  let query = Query::nodes(
    NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
      field("metadata.type")
        .eq(IdentityType::User.as_str())
        .into(),
      field("metadata.name").eq(username).into(),
    ])),
  );

  let elements = repo
    .query(query)
    .await
    .map_err(|e| Status::internal(format!("failed to query user: {}", e)))?;

  for el in elements {
    if let Element::Node(node) = el
      && let mises_core::model::node::NodeMeta::Identity(IdentityMeta::User {
        name,
        encrypted_password,
        ..
      }) = &node.metadata
      && name == username
    {
      // Verify the password hash
      let is_valid = password::verify_password(password_plain, encrypted_password)
        .map_err(|e| Status::internal(format!("failed to verify password: {}", e)))?;

      if is_valid {
        return Ok(node);
      }
    }
  }

  Err(Status::unauthenticated("invalid username or password"))
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

async fn get_signing_key<R>(repo: &R, identity_id: uuid::Uuid) -> Result<Vec<u8>, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  use base64::Engine;

  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::Owns.as_str())
      .from(NodeQuery::any().filter(field("id").eq(identity_id.to_string()))),
  );

  let elements = repo
    .query(query)
    .await
    .map_err(|e| Status::internal(format!("failed to query key edges: {}", e)))?;

  for el in elements.iter() {
    if let Element::Edge(edge) = el {
      if let Ok(Some(key_node)) = repo.get_node_by_id(edge.to_id).await {
        if let NodeMeta::Key(key_meta) = &key_node.metadata {
          if let Some(private_key_b64) = &key_meta.private_key {
            // Try different base64 decoders
            let decode_result = base64::engine::general_purpose::URL_SAFE_NO_PAD
              .decode(private_key_b64.as_bytes())
              .or_else(|_| {
                base64::engine::general_purpose::STANDARD.decode(private_key_b64.as_bytes())
              })
              .or_else(|_| {
                base64::engine::general_purpose::URL_SAFE.decode(private_key_b64.as_bytes())
              });

            if let Ok(private_key_bytes) = decode_result {
              return Ok(private_key_bytes);
            }
          }
        }
      }
    }
  }

  Err(Status::internal(format!(
    "signing key not found for identity: {}",
    identity_id
  )))
}
