use tonic::Status;
use uuid::Uuid;

use mises_core::{
  CoreError, model::identity::IdentityType, service::identity::IdentityService, traits::Repository,
};

pub async fn resolve_client_id<R>(client_id: &str, repo: R) -> Result<Uuid, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  if client_id.trim() == "tauri" || client_id.trim() == "bootstrap" {
    let identity_service = IdentityService::new(repo);
    let app_node = identity_service
      .find_any_application()
      .await
      .map_err(|e| Status::internal(format!("identity service error: {}", e)))?;

    app_node.map(|node| node.id).ok_or_else(|| {
      Status::invalid_argument("invalid_request: no application client found for 'tauri'")
    })
  } else {
    Uuid::parse_str(client_id.trim())
      .map_err(|_| Status::invalid_argument(format!("invalid client_id: {}", client_id)))
  }
}

pub async fn ensure_application_identity<R>(client_uuid: Uuid, repo: R) -> Result<R::Node, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo);
  identity_service
    .get_node_by_id_and_identity_type(client_uuid, IdentityType::Application)
    .await
    .map_err(|e| match e {
      CoreError::NotFound => Status::invalid_argument(format!(
        "invalid_request: client_id not found: {}",
        client_uuid
      )),
      CoreError::InvalidInput(_) => {
        Status::invalid_argument("invalid_request: client_id does not refer to an application")
      }
      _ => Status::internal(format!("identity service error: {}", e)),
    })
}
