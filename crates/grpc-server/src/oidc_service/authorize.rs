use tonic::Status;

use mises_core::{
  CoreError,
  model::{
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
  },
  service::identity::IdentityService,
  traits::Repository,
};
use uuid::Uuid;

use crate::oidc_service::constants::{self, CODE_CHALLENGE_METHODS_SUPPORTED};

/// Perform authorize request validation and resolution. Returns an
/// `AuthorizeResponse` suitable for returning from the gRPC service.
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

  let client_uuid = match Uuid::parse_str(req.client_id.trim()) {
    Ok(u) => u,
    Err(_) => {
      return Err(Status::invalid_argument(format!(
        "invalid client_id: {}",
        req.client_id
      )));
    }
  };

  let identity_service = IdentityService::new(repo.clone());

  let node = match identity_service
    .get_node_by_id_and_identity_type(client_uuid, IdentityType::Application)
    .await
  {
    Ok(n) => n,
    Err(e) => {
      return match e {
        CoreError::NotFound => Err(Status::invalid_argument(format!(
          "invalid_request: client_id not found: {}",
          req.client_id
        ))),
        CoreError::InvalidInput(_) => Err(Status::invalid_argument(
          "invalid_request: client_id does not refer to an application",
        )),
        _ => Err(Status::internal(format!("identity service error: {}", e))),
      };
    }
  };

  // get OIDC client registration metadata
  let oidc_meta = match &node.metadata {
    NodeMeta::Identity(IdentityMeta::Application { oidc_client, .. }) => oidc_client.as_ref(),
    _ => None,
  };

  // require openid scope
  if let Some(ref scope) = req.scope
    && !scope
      .split_whitespace()
      .any(|s| s == constants::SCOPE_OPENID)
  {
    return Err(Status::invalid_argument(
      constants::ERR_SCOPE_MUST_INCLUDE_OPENID,
    ));
  }

  // resolve and validate redirect_uri against the application's oidc metadata
  let resolved_redirect = if let Some(client) = oidc_meta {
    // reject registration when client_id is provided
    if req.registration.is_some() {
      return Err(Status::invalid_argument(
        "invalid_request: registration parameter not allowed when client_id is provided",
      ));
    }

    // validate response_mode
    if let Some(ref mode) = req.response_mode
      && constants::RESPONSE_MODES.iter().all(|&m| m != mode)
    {
      return Err(Status::invalid_argument(
        "invalid_request: response_mode not supported",
      ));
    }

    // PKCE: require for public clients on code flow
    let code_requested = req
      .response_type
      .split_whitespace()
      .any(|s| s == constants::RESPONSE_TYPE_CODE);
    // public client when token_endpoint_auth_method == "none" or missing
    let client_is_public = client
      .token_endpoint_auth_method
      .as_deref()
      .map(|s| s == "none")
      .unwrap_or(false);

    if code_requested {
      // if client is public, require a code_challenge
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

      // if a code_challenge is present, validate it and require S256 explicitly
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
            // require S256 for code_challenge_method
            return Err(Status::invalid_argument(
              "invalid_request: code_challenge_method required and must be 'S256'",
            ));
          }
        }
      }
    }
    // code_challenge disallowed when not code flow
    else if req.code_challenge.is_some() {
      return Err(Status::invalid_argument(
        "invalid_request: code_challenge only allowed with response_type including 'code'",
      ));
    }

    // nonce required when id_token requested
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

    // validate scopes against client metadata
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
    // require `oidc_client` metadata
    return Err(Status::invalid_argument(
      "invalid_request: oidc_client metadata missing",
    ));
  };

  // ensure redirect is a valid URL
  if url::Url::parse(&resolved_redirect).is_err() {
    return Err(Status::invalid_argument(format!(
      "invalid redirect_uri: {}",
      resolved_redirect
    )));
  }

  // ensure client has registered allowed response_types
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
    // client must register allowed response_types
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

    // create application identity with registered redirect
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

  // Remaining tests copied and adapted from the service tests

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
}
