use mises_graph::KeyValueStoreExecutor;
use tonic::Status;

use mises_core::{
  model::{identity::IdentityMeta, node::NodeMeta},
  traits::Repository,
};
use uuid::Uuid;

use crate::jwt::Claims;
use crate::oidc_service::{
  authorization_code::{
    AuthorizationCodeData, generate_authorization_code, store_authorization_code,
  },
  constants::{self, CODE_CHALLENGE_METHODS_SUPPORTED},
  helpers::{ensure_application_identity, matches_redirect_pattern},
};

struct AuthorizeError {
  error: String,
  error_description: Option<String>,
  redirect_uri: String,
}

impl AuthorizeError {
  fn new(error: &str, description: &str, redirect_uri: String) -> Self {
    Self {
      error: error.to_string(),
      error_description: Some(description.to_string()),
      redirect_uri,
    }
  }

  fn to_response(&self) -> Result<mises_proto::AuthorizeResponse, Status> {
    let mut url = url::Url::parse(&self.redirect_uri)
      .map_err(|_| Status::internal("failed to parse redirect url"))?;
    {
      let mut query_pairs = url.query_pairs_mut();
      query_pairs.append_pair("error", &self.error);
      if let Some(ref desc) = self.error_description {
        query_pairs.append_pair("error_description", desc);
      }
    }
    Ok(mises_proto::AuthorizeResponse {
      redirect_uri: Some(url.to_string()),
    })
  }
}

pub async fn authorize<R, S>(
  repo: &R,
  store: &S,
  req: mises_proto::AuthorizeRequest,
  claims: Option<Claims>,
  sign_in_url: &Option<String>,
) -> Result<mises_proto::AuthorizeResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
  S: KeyValueStoreExecutor,
{
  if claims.is_none() {
    if let Some(sign_in_url_str) = sign_in_url {
      let mut sign_in_redirect =
        url::Url::parse(sign_in_url_str).map_err(|_| Status::internal("invalid sign-in url"))?;
      {
        let mut query_pairs = sign_in_redirect.query_pairs_mut();
        if !req.client_id.trim().is_empty() {
          query_pairs.append_pair("client_id", &req.client_id);
        }
        if !req.response_type.trim().is_empty() {
          query_pairs.append_pair("response_type", &req.response_type);
        }
        if let Some(ref redirect_uri) = req.redirect_uri
          && !redirect_uri.trim().is_empty()
        {
          query_pairs.append_pair("redirect_uri", redirect_uri);
        }
        if let Some(ref scope) = req.scope
          && !scope.trim().is_empty()
        {
          query_pairs.append_pair("scope", scope);
        }
        if let Some(ref nonce) = req.nonce
          && !nonce.trim().is_empty()
        {
          query_pairs.append_pair("nonce", nonce);
        }
      }
      return Ok(mises_proto::AuthorizeResponse {
        redirect_uri: Some(sign_in_redirect.to_string()),
      });
    }
    return Err(Status::unauthenticated(
      "authorization required: bearer token not provided",
    ));
  }

  if let Some(redirect) = &req.redirect_uri {
    if redirect.trim().is_empty() {
      return Err(Status::invalid_argument(constants::ERR_REDIRECT_URI_EMPTY));
    }
    if url::Url::parse(redirect).is_err() {
      return Err(Status::invalid_argument(format!(
        "invalid redirect_uri: {}",
        redirect
      )));
    }
  }

  if req.client_id.trim().is_empty() {
    return Err(Status::invalid_argument("client_id is required"));
  }

  if req.response_type.trim().is_empty() {
    return Err(Status::invalid_argument("response_type is required"));
  }

  let client_uuid = match Uuid::parse_str(&req.client_id) {
    Ok(id) => id,
    Err(e) => {
      return Err(Status::invalid_argument(format!(
        "invalid client id: {}",
        e
      )));
    }
  };

  let node = match ensure_application_identity(repo, client_uuid).await {
    Ok(n) => n,
    Err(e) => return Err(e),
  };

  let client = match node.metadata {
    NodeMeta::Identity(IdentityMeta::Application {
      oidc: Some(oidc), ..
    }) => oidc,
    _ => {
      return Err(Status::invalid_argument(
        "invalid client_id: not an OIDC client",
      ));
    }
  };

  if req.registration.is_some() {
    return Err(Status::invalid_argument(
      "registration parameter not allowed when client_id is provided",
    ));
  }

  if let Some(ref mode) = req.response_mode
    && constants::RESPONSE_MODES.iter().all(|&m| m != mode)
  {
    return Err(Status::invalid_argument("response_mode not supported"));
  }

  if let Some(ref scope) = req.scope
    && !scope
      .split_whitespace()
      .any(|s| s == constants::SCOPE_OPENID)
  {
    return Err(Status::invalid_argument(
      constants::ERR_SCOPE_MUST_INCLUDE_OPENID,
    ));
  }

  let resolved_redirect = if let Some(ref redirect) = req.redirect_uri {
    if redirect.trim().is_empty() {
      return Err(Status::invalid_argument(constants::ERR_REDIRECT_URI_EMPTY));
    }
    if !client
      .redirect_uris
      .iter()
      .any(|pattern| matches_redirect_pattern(redirect, pattern))
    {
      return Err(Status::invalid_argument("redirect_uri mismatch"));
    }
    redirect.clone()
  } else if client.redirect_uris.len() == 1 {
    client.redirect_uris[0].clone()
  } else {
    return Err(Status::invalid_argument(
      "redirect_uri is required for this client",
    ));
  };

  if url::Url::parse(&resolved_redirect).is_err() {
    let err = AuthorizeError::new(
      "invalid_request",
      &format!("invalid redirect_uri: {}", resolved_redirect),
      resolved_redirect,
    );
    return err.to_response();
  }

  let code_requested = req
    .response_type
    .split_whitespace()
    .any(|s| s == constants::RESPONSE_TYPE_CODE);
  let client_is_public = matches!(
    client.token_endpoint_auth_method,
    Some(mises_core::model::oidc::TokenEndpointAuthMethod::None)
  );

  if code_requested {
    if client_is_public
      && req
        .code_challenge
        .as_ref()
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
      let err = AuthorizeError::new(
        "invalid_request",
        "code_challenge is required for public clients using 'code' response_type",
        resolved_redirect,
      );
      return err.to_response();
    }

    if let Some(ref cc) = req.code_challenge {
      if cc.trim().is_empty() {
        let err = AuthorizeError::new(
          "invalid_request",
          "code_challenge provided is empty",
          resolved_redirect,
        );
        return err.to_response();
      }

      match req.code_challenge_method.as_deref() {
        Some(x) if CODE_CHALLENGE_METHODS_SUPPORTED.contains(&x) => {}
        Some(_) => {
          let err = AuthorizeError::new(
            "invalid_request",
            "code_challenge_method not supported",
            resolved_redirect,
          );
          return err.to_response();
        }
        None => {
          let err = AuthorizeError::new(
            "invalid_request",
            "code_challenge_method required and must be 'S256'",
            resolved_redirect,
          );
          return err.to_response();
        }
      }
    }
  } else if req.code_challenge.is_some() {
    let err = AuthorizeError::new(
      "invalid_request",
      "code_challenge only allowed with response_type including 'code'",
      resolved_redirect,
    );
    return err.to_response();
  }

  if req
    .response_type
    .split_whitespace()
    .any(|s| s == constants::RESPONSE_TYPE_ID_TOKEN)
    && req
      .nonce
      .as_ref()
      .map(|n| n.trim().is_empty())
      .unwrap_or(true)
  {
    let err = AuthorizeError::new(
      "invalid_request",
      "nonce is required when 'id_token' is requested",
      resolved_redirect,
    );
    return err.to_response();
  }

  if let Some(ref scope) = req.scope
    && let Some(ref allowed) = client.scope
    && !allowed.trim().is_empty()
  {
    for s in scope.split_whitespace() {
      if !allowed.split_whitespace().any(|cs| cs == s) {
        let err = AuthorizeError::new(
          "invalid_request",
          "scope contains values not allowed for client",
          resolved_redirect,
        );
        return err.to_response();
      }
    }
  }

  if client.response_types.is_empty() {
    let err = AuthorizeError::new(
      "server_error",
      "oidc_client has no registered response_types",
      resolved_redirect,
    );
    return err.to_response();
  }

  for part in req.response_type.split_whitespace() {
    if !client.response_types.iter().any(|rt| rt.as_str() == part) {
      let err = AuthorizeError::new(
        "unauthorized_client",
        &format!("response_type '{}' not allowed for client", part),
        resolved_redirect,
      );
      return err.to_response();
    }
  }

  let subject = claims
    .as_ref()
    .ok_or_else(|| Status::unauthenticated("authentication required"))?
    .sub
    .parse::<Uuid>()
    .map_err(|_| Status::invalid_argument("invalid subject UUID"))?;

  let code = generate_authorization_code();

  let authorization_data = AuthorizationCodeData::new(
    client_uuid,
    subject,
    resolved_redirect.clone(),
    req.scope.clone(),
    req.nonce.clone(),
    req.code_challenge.clone(),
    req.code_challenge_method.clone(),
  );

  store_authorization_code(store, &code, authorization_data).await?;

  let mut final_redirect = url::Url::parse(&resolved_redirect)
    .map_err(|_| Status::internal("failed to parse redirect uri"))?;
  {
    let mut query_pairs = final_redirect.query_pairs_mut();
    query_pairs.append_pair("code", &code);
    if let Some(ref state) = req.state {
      query_pairs.append_pair("state", state);
    }
  }

  Ok(mises_proto::AuthorizeResponse {
    redirect_uri: Some(final_redirect.to_string()),
  })
}
