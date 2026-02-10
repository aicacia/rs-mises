use tonic::Status;

use mises_core::{
  model::{identity::IdentityMeta, node::NodeMeta},
  traits::Repository,
};

use crate::oidc_service::{
  constants::{self, CODE_CHALLENGE_METHODS_SUPPORTED},
  helpers::{ensure_application_identity, resolve_client_id},
};

pub async fn authorize<R>(
  repo: R,
  req: mises_proto::AuthorizeRequest,
) -> Result<mises_proto::AuthorizeResponse, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  if req.client_id.trim().is_empty() {
    return Err(Status::invalid_argument("client_id is required"));
  }

  if req.response_type.trim().is_empty() {
    return Err(Status::invalid_argument("response_type is required"));
  }

  if let Some(ref redirect) = req.redirect_uri {
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

  let client_uuid = resolve_client_id(&req.client_id, repo.clone()).await?;
  let node = ensure_application_identity(client_uuid, repo.clone()).await?;

  let oidc_meta = match &node.metadata {
    NodeMeta::Identity(IdentityMeta::Application { oidc_client, .. }) => oidc_client.as_ref(),
    _ => None,
  };

  if let Some(ref scope) = req.scope
    && !scope
      .split_whitespace()
      .any(|s| s == constants::SCOPE_OPENID)
  {
    return Err(Status::invalid_argument(
      constants::ERR_SCOPE_MUST_INCLUDE_OPENID,
    ));
  }

  let resolved_redirect = if let Some(client) = oidc_meta {
    if req.registration.is_some() {
      return Err(Status::invalid_argument(
        "invalid_request: registration parameter not allowed when client_id is provided",
      ));
    }

    if let Some(ref mode) = req.response_mode
      && constants::RESPONSE_MODES.iter().all(|&m| m != mode)
    {
      return Err(Status::invalid_argument(
        "invalid_request: response_mode not supported",
      ));
    }

    let code_requested = req
      .response_type
      .split_whitespace()
      .any(|s| s == constants::RESPONSE_TYPE_CODE);
    let client_is_public = client
      .token_endpoint_auth_method
      .as_deref()
      .map(|s| s == "none")
      .unwrap_or(false);

    if code_requested {
      if client_is_public
        && req
          .code_challenge
          .as_ref()
          .map(|s| s.trim().is_empty())
          .unwrap_or(true)
      {
        return Err(Status::invalid_argument(
          "invalid_request: code_challenge is required for public clients using 'code' response_type",
        ));
      }

      if let Some(ref cc) = req.code_challenge {
        if cc.trim().is_empty() {
          return Err(Status::invalid_argument(
            "invalid_request: code_challenge provided is empty",
          ));
        }

        match req.code_challenge_method.as_deref() {
          Some(x) if CODE_CHALLENGE_METHODS_SUPPORTED.contains(&x) => {}
          Some(_) => {
            return Err(Status::invalid_argument(
              "invalid_request: code_challenge_method not supported",
            ));
          }
          None => {
            return Err(Status::invalid_argument(
              "invalid_request: code_challenge_method required and must be 'S256'",
            ));
          }
        }
      }
    } else if req.code_challenge.is_some() {
      return Err(Status::invalid_argument(
        "invalid_request: code_challenge only allowed with response_type including 'code'",
      ));
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
      return Err(Status::invalid_argument(
        "invalid_request: nonce is required when 'id_token' is requested",
      ));
    }

    if let Some(ref scope) = req.scope
      && !client.scopes.is_empty()
    {
      for s in scope.split_whitespace() {
        if !client.scopes.iter().any(|cs| cs == s) {
          return Err(Status::invalid_argument(
            "invalid_request: scope contains values not allowed for client",
          ));
        }
      }
    }

    if let Some(ref redirect) = req.redirect_uri {
      if redirect.trim().is_empty() {
        return Err(Status::invalid_argument(constants::ERR_REDIRECT_URI_EMPTY));
      }
      if !client.redirect_uris.iter().any(|r| r == redirect) {
        return Err(Status::invalid_argument(
          "unauthorized_client: redirect_uri mismatch",
        ));
      }
      redirect.clone()
    } else if client.redirect_uris.len() == 1 {
      client.redirect_uris[0].clone()
    } else {
      return Err(Status::invalid_argument(
        "invalid_request: redirect_uri is required for this client",
      ));
    }
  } else {
    return Err(Status::invalid_argument(
      "invalid_request: oidc_client metadata missing",
    ));
  };

  if url::Url::parse(&resolved_redirect).is_err() {
    return Err(Status::invalid_argument(format!(
      "invalid redirect_uri: {}",
      resolved_redirect
    )));
  }

  if let Some(client) = oidc_meta {
    if client.response_types.is_empty() {
      return Err(Status::invalid_argument(
        "invalid_request: oidc_client has no registered response_types",
      ));
    }
    for part in req.response_type.split_whitespace() {
      if !client.response_types.iter().any(|rt| rt == part) {
        return Err(Status::permission_denied(format!(
          "unauthorized_client: response_type '{}' not allowed for client",
          part
        )));
      }
    }
  } else {
    return Err(Status::invalid_argument(
      "invalid_request: oidc_client metadata missing",
    ));
  }

  Ok(mises_proto::AuthorizeResponse {
    redirect_uri: Some(resolved_redirect),
  })
}

#[cfg(test)]
mod tests {
  use crate::oidc_service::constants;

  use super::authorize;
  use mises_core::model::edge::EdgeProps;
  use mises_core::model::identity::ApplicationMeta;
  use mises_core::model::identity::IdentityMeta;
  use mises_core::model::node::NodeMeta;
  use mises_graph::{Executor, IdGenerator, InMemoryKeyValueStore, KeyValueRepository};
  use mises_proto::oidc_service_server::OidcService;
  use uuid::Uuid;

  #[derive(Clone)]
  struct UuidGenerator;
  impl IdGenerator<Uuid> for UuidGenerator {
    fn next(&self) -> Uuid {
      Uuid::new_v4()
    }
  }

  fn make_repo()
  -> KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore> {
    KeyValueRepository::new(InMemoryKeyValueStore::new(), UuidGenerator)
  }

  #[tokio::test]
  async fn authorize_redirect_uri_mismatch() {
    let repo = make_repo();

    let app_id = repo
      .create_node(
        "identity".to_string(),
        NodeMeta::Identity(IdentityMeta::Application {
          name: "app".to_string(),
          local: true,
          oidc_client: Some(ApplicationMeta {
            redirect_uris: vec!["https://example.com/callback".to_string()],
            response_types: vec![constants::RESPONSE_TYPE_CODE.to_string()],
            grant_types: vec![],
            scopes: vec![constants::SCOPE_OPENID.to_string()],
            token_endpoint_auth_method: None,
          }),
        }),
      )
      .await
      .unwrap()
      .id;

    let req = mises_proto::AuthorizeRequest {
      client_id: app_id.to_string(),
      response_type: constants::RESPONSE_TYPE_CODE.to_string(),
      response_mode: None,
      scope: Some(constants::SCOPE_OPENID.to_string()),
      redirect_uri: Some("https://evil.com".to_string()),
      state: None,
      nonce: None,
      registration: None,
      code_challenge: None,
      code_challenge_method: None,
    };

    let res = authorize(repo, req).await;
    assert!(res.is_err());
    let e = res.err().unwrap();
    assert!(e.message().contains("redirect_uri mismatch"));
  }

  #[tokio::test]
  async fn authorize_missing_redirect_uri_single_registered() {
    let repo = make_repo();

    let app_id = repo
      .create_node(
        "identity".to_string(),
        NodeMeta::Identity(IdentityMeta::Application {
          name: "app".to_string(),
          local: true,
          oidc_client: Some(ApplicationMeta {
            redirect_uris: vec!["https://example.com/callback".to_string()],
            response_types: vec![constants::RESPONSE_TYPE_CODE.to_string()],
            grant_types: vec![],
            scopes: vec![constants::SCOPE_OPENID.to_string()],
            token_endpoint_auth_method: Some("none".to_string()),
          }),
        }),
      )
      .await
      .unwrap()
      .id;

    let req = mises_proto::AuthorizeRequest {
      client_id: app_id.to_string(),
      response_type: constants::RESPONSE_TYPE_CODE.to_string(),
      response_mode: None,
      scope: Some(constants::SCOPE_OPENID.to_string()),
      redirect_uri: None,
      state: None,
      nonce: None,
      registration: None,
      code_challenge: Some("abc".to_string()),
      code_challenge_method: Some("S256".to_string()),
    };

    let res = authorize(repo, req).await;
    assert!(res.is_ok());
    let r = res.ok().unwrap();
    assert_eq!(
      r.redirect_uri,
      Some("https://example.com/callback".to_string())
    );
  }

  #[tokio::test]
  async fn authorize_scope_requires_openid() {
    let repo = make_repo();

    let app_id = repo
      .create_node(
        "identity".to_string(),
        NodeMeta::Identity(IdentityMeta::Application {
          name: "app".to_string(),
          local: true,
          oidc_client: Some(ApplicationMeta {
            redirect_uris: vec!["https://example.com/callback".to_string()],
            response_types: vec![constants::RESPONSE_TYPE_CODE.to_string()],
            grant_types: vec![],
            scopes: vec![constants::SCOPE_OPENID.to_string()],
            token_endpoint_auth_method: Some("none".to_string()),
          }),
        }),
      )
      .await
      .unwrap()
      .id;

    let req = mises_proto::AuthorizeRequest {
      client_id: app_id.to_string(),
      response_type: constants::RESPONSE_TYPE_CODE.to_string(),
      response_mode: None,
      scope: Some("profile".to_string()),
      redirect_uri: Some("https://example.com/callback".to_string()),
      state: None,
      nonce: None,
      registration: None,
      code_challenge: None,
      code_challenge_method: None,
    };

    let res = authorize(repo, req).await;
    assert!(res.is_err());
    let e = res.err().unwrap();
    assert!(e.message().contains("scope must include 'openid'"));
  }

  #[tokio::test]
  async fn native_authenticate_issues_valid_id_token() {
    let repo = make_repo();

    let app_id = repo
      .create_node(
        "identity".to_string(),
        NodeMeta::Identity(IdentityMeta::Application {
          name: "app".to_string(),
          local: true,
          oidc_client: None,
        }),
      )
      .await
      .unwrap()
      .id;

    let issuer = "https://example.com".to_string();
    let hmac = "test-secret".to_string();

    let svc = crate::oidc_service::service::OidcService::new(
      repo.clone(),
      issuer.clone(),
      None,
      Some(hmac.clone()),
    );

    let req = mises_proto::NativeAuthenticateRequest {
      client_id: Some(app_id.to_string()),
      sub: None,
      scope: Some(constants::SCOPE_OPENID.to_string()),
    };

    let res = svc
      .native_authenticate(tonic::Request::new(req))
      .await
      .unwrap()
      .into_inner();
    assert!(res.id_token.is_some());
    let tok = res.id_token.unwrap();

    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    let token_data: jsonwebtoken::TokenData<serde_json::Value> = jsonwebtoken::decode(
      &tok,
      &jsonwebtoken::DecodingKey::from_secret(hmac.as_bytes()),
      &validation,
    )
    .expect("token decode");

    let claims = token_data.claims;
    assert_eq!(
      claims.get("iss").and_then(|v| v.as_str()),
      Some(issuer.as_str())
    );
    assert_eq!(
      claims.get("aud").and_then(|v| v.as_str()),
      Some(app_id.to_string().as_str())
    );
    assert!(claims.get("sub").and_then(|v| v.as_str()).is_some());
    assert!(claims.get("exp").and_then(|v| v.as_i64()).is_some());
  }
}
