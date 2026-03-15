use tonic::Status;
use uuid::Uuid;

use mises_core::{
  error::CoreError,
  model::{
    identity::{IdentityMeta, IdentityType},
    node::NodeMeta,
    oidc::ApplicationType,
  },
  service::identity::IdentityService,
  traits::Repository,
};

use crate::{
  error::ToStatus,
  helpers::{OptionExt, ResultExt},
  oidc_service::helpers::ensure_service_owns_application,
};

pub async fn client_register<R>(
  repo: &R,
  device_id: &str,
  request: mises_proto::ClientRegisterRequest,
) -> Result<mises_proto::Client, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  fn optional_string(value: &str) -> Option<String> {
    if value.is_empty() {
      None
    } else {
      Some(value.to_string())
    }
  }

  fn parse_application_type(value: &str) -> Option<ApplicationType> {
    match value {
      "web" => Some(ApplicationType::Web),
      "native" => Some(ApplicationType::Native),
      _ => None,
    }
  }

  fn application_type_to_string(application_type: ApplicationType) -> String {
    match application_type {
      ApplicationType::Web => "web".to_string(),
      ApplicationType::Native => "native".to_string(),
    }
  }

  let client_id = request
    .client_id
    .and_then(|id| if id.trim().is_empty() { None } else { Some(id) });

  let service_id = request
    .service_id
    .clone()
    .filter(|id| !id.trim().is_empty())
    .or_invalid_argument("missing service_id")?;

  let identity_service = IdentityService::new(repo.clone(), device_id.to_string());

  let service_node = match identity_service
    .find_service_by_name(&service_id)
    .await
    .map_err(|e| e.to_status())?
  {
    Some(node) => node,
    None => {
      let (node, _key) = identity_service
        .create_service(service_id.clone(), None)
        .await
        .map_err(|e| e.to_status())?;
      node
    }
  };
  let service_uuid = service_node.id;

  let is_new_client;
  let app_node = if let Some(ref id_str) = client_id {
    let client_uuid = Uuid::parse_str(id_str).or_invalid_argument("invalid client_id format")?;

    match identity_service
      .get_node_by_id_and_identity_type(client_uuid, IdentityType::Application)
      .await
    {
      Ok(existing_node) => {
        ensure_service_owns_application(&identity_service, service_uuid, existing_node.id).await?;
        is_new_client = false;
        existing_node
      }
      Err(CoreError::NotFound) => {
        let app_name = request
          .name
          .clone()
          .filter(|n| !n.trim().is_empty())
          .unwrap_or_else(|| "OIDC Client".to_string());

        let mut oidc_meta = mises_core::model::oidc::OidcClientMeta {
          client_name: app_name.clone(),
          ..Default::default()
        };
        oidc_meta.client_id = client_uuid.to_string();

        let (app_node, _key_node) = identity_service
          .create_application(Some(client_uuid), oidc_meta)
          .await
          .map_err(|e| e.to_status())?;

        identity_service
          .set_owner(service_uuid, app_node.id)
          .await
          .map_err(|e| e.to_status())?;

        is_new_client = true;
        app_node
      }
      Err(CoreError::InvalidInput(_)) => {
        return Err(Status::invalid_argument(
          "invalid_request: client_id does not refer to an application",
        ));
      }
      Err(e) => return Err(Status::internal(format!("identity service error: {}", e))),
    }
  } else {
    let app_name = request
      .name
      .clone()
      .filter(|n| !n.trim().is_empty())
      .unwrap_or_else(|| "OIDC Client".to_string());

    let oidc_meta = mises_core::model::oidc::OidcClientMeta {
      client_name: app_name.clone(),
      ..Default::default()
    };

    let (app_node, _key_node) = identity_service
      .create_application(None, oidc_meta)
      .await
      .map_err(|e| e.to_status())?;

    identity_service
      .set_owner(service_uuid, app_node.id)
      .await
      .map_err(|e| e.to_status())?;

    is_new_client = true;
    app_node
  };

  let app_id = app_node.id;

  let current_oidc = if let NodeMeta::Identity(identity_meta) = &app_node.metadata {
    if let IdentityMeta::Application { oidc } = identity_meta.as_ref() {
      oidc.as_ref().clone()
    } else {
      Default::default()
    }
  } else {
    Default::default()
  };

  let mut oidc_meta = current_oidc;

  if is_new_client {
    if oidc_meta.client_id.is_empty() {
      oidc_meta.client_id = app_id.to_string();
    }
    oidc_meta.client_secret = Uuid::new_v4().to_string();
  }

  if !request.redirect_uris.is_empty() {
    oidc_meta.redirect_uris = request.redirect_uris;
  }

  if !request.grant_types.is_empty() {
    oidc_meta.grant_types = request
      .grant_types
      .into_iter()
      .filter_map(|gt| gt.parse().ok())
      .collect();
  }

  if !request.response_types.is_empty() {
    oidc_meta.response_types = request
      .response_types
      .into_iter()
      .filter_map(|rt| rt.parse().ok())
      .collect();
  }

  if let Some(scope) = request.scope.filter(|s| !s.trim().is_empty()) {
    oidc_meta.scope = scope;
  }

  if let Some(auth_method) = request
    .token_endpoint_auth_method
    .filter(|m| !m.trim().is_empty())
    && let Ok(method) = auth_method.parse()
  {
    oidc_meta.token_endpoint_auth_method = method;
  }

  if let Some(application_type) = request
    .application_type
    .as_deref()
    .map(str::trim)
    .and_then(parse_application_type)
  {
    oidc_meta.application_type = application_type;
  }

  if let Some(require_pkce) = request.require_pkce {
    oidc_meta.require_pkce = require_pkce;
  }

  if !request.contacts.is_empty() {
    oidc_meta.contacts = request.contacts;
  }

  if let Some(client_uri) = request.client_uri.filter(|uri| !uri.trim().is_empty()) {
    oidc_meta.client_uri = client_uri;
  }

  if let Some(logo_uri) = request.logo_uri.filter(|uri| !uri.trim().is_empty()) {
    oidc_meta.logo_uri = logo_uri;
  }

  if !request.post_logout_redirect_uris.is_empty() {
    oidc_meta.post_logout_redirect_uris = request.post_logout_redirect_uris;
  }

  if let Some(policy_uri) = request.policy_uri.filter(|uri| !uri.trim().is_empty()) {
    oidc_meta.policy_uri = policy_uri;
  }

  if let Some(tos_uri) = request.tos_uri.filter(|uri| !uri.trim().is_empty()) {
    oidc_meta.tos_uri = tos_uri;
  }

  if let Some(access_token_expiry) = request.access_token_expiry {
    oidc_meta.access_token_expiry = access_token_expiry;
  }

  if let Some(refresh_token_expiry) = request.refresh_token_expiry {
    oidc_meta.refresh_token_expiry = refresh_token_expiry;
  }

  oidc_meta.service_id = service_id;

  if let Some(name) = request.name.filter(|n| !n.trim().is_empty()) {
    oidc_meta.client_name = name;
  }

  let updated_identity = IdentityMeta::Application {
    oidc: Box::new(oidc_meta.clone()),
  };

  repo
    .update_node(app_id, NodeMeta::Identity(Box::new(updated_identity)), None)
    .await
    .map_err(|e| e.to_status())?;

  Ok(mises_proto::Client {
    id: app_id.to_string(),
    client_id: if oidc_meta.client_id.is_empty() {
      app_id.to_string()
    } else {
      oidc_meta.client_id
    },
    client_secret: if oidc_meta.client_secret.is_empty() {
      None
    } else {
      Some(oidc_meta.client_secret)
    },
    name: if oidc_meta.client_name.is_empty() {
      None
    } else {
      Some(oidc_meta.client_name)
    },
    redirect_uris: oidc_meta.redirect_uris,
    grant_types: oidc_meta
      .grant_types
      .iter()
      .map(|gt: &mises_core::model::oidc::GrantType| gt.as_str().to_string())
      .collect(),
    response_types: oidc_meta
      .response_types
      .iter()
      .map(|rt: &mises_core::model::oidc::ResponseType| rt.as_str().to_string())
      .collect(),
    scope: if oidc_meta.scope.is_empty() {
      None
    } else {
      Some(oidc_meta.scope)
    },
    token_endpoint_auth_method: Some(oidc_meta.token_endpoint_auth_method.as_str().to_string()),
    require_pkce: Some(oidc_meta.require_pkce),
    application_type: Some(application_type_to_string(oidc_meta.application_type)),
    contacts: oidc_meta.contacts,
    service_id: Some(oidc_meta.service_id.clone()),
    client_uri: optional_string(&oidc_meta.client_uri),
    logo_uri: optional_string(&oidc_meta.logo_uri),
    policy_uri: if oidc_meta.policy_uri.is_empty() {
      None
    } else {
      Some(oidc_meta.policy_uri)
    },
    tos_uri: if oidc_meta.tos_uri.is_empty() {
      None
    } else {
      Some(oidc_meta.tos_uri)
    },
    jwks_uri: None,
    jwks: None,
    sector_identifier_uri: None,
    subject_type: None,
    id_token_signed_response_alg: None,
    id_token_encrypted_response_alg: None,
    id_token_encrypted_response_enc: None,
    userinfo_signed_response_alg: None,
    userinfo_encrypted_response_alg: None,
    userinfo_encrypted_response_enc: None,
    request_object_signing_alg: None,
    request_object_encryption_alg: None,
    request_object_encryption_enc: None,
    token_endpoint_auth_signing_alg: None,
    default_max_age: None,
    require_auth_time: None,
    default_acr_values: vec![],
    initiate_login_uri: None,
    request_uris: vec![],
    post_logout_redirect_uris: oidc_meta.post_logout_redirect_uris,
    frontchannel_logout_uri: None,
    frontchannel_logout_session_required: None,
    backchannel_logout_uri: None,
    backchannel_logout_session_required: None,
    access_token_expiry: Some(oidc_meta.access_token_expiry),
    refresh_token_expiry: Some(oidc_meta.refresh_token_expiry),
  })
}
