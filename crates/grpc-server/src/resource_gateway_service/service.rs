use std::collections::HashMap;

use tonic::{Request, Response, Status};
use url::Url;
use uuid::Uuid;

use mises_core::{service::identity::IdentityService, traits::Repository};

use crate::{
  helpers::{OptionExt, ResultExt, parse_uuid},
  jwt::{Claims, extract_optional_claims},
};

pub struct ResourceGatewayService<R>
where
  R: Repository,
{
  repo: R,
  device_id: String,
  gateway_base_uri: Url,
}

impl<R> ResourceGatewayService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  pub fn new(repo: R, device_id: String, gateway_base_uri: Url) -> Self {
    Self {
      repo,
      device_id,
      gateway_base_uri,
    }
  }

  fn identity_from_claims(claims: &Claims) -> Result<Uuid, Status> {
    if let Some(acting_for) = &claims.acting_for {
      return parse_uuid(acting_for);
    }

    parse_uuid(&claims.sub)
  }

  fn protocol_for_uri(&self) -> Option<i32> {
    match self.gateway_base_uri.scheme() {
      "http" => Some(1),
      "https" => Some(2),
      _ => None,
    }
  }

  fn endpoint_for_resource(
    &self,
    resource_id: Uuid,
    resource_type: &str,
  ) -> Result<Option<mises_proto::AccessEndpoint>, Status> {
    let Some(protocol) = self.protocol_for_uri() else {
      return Ok(None);
    };

    let path = if resource_type == "file-system" {
      format!("resources/{resource_id}/files")
    } else {
      format!("resources/{resource_id}")
    };

    let address = self
      .gateway_base_uri
      .join(&path)
      .map_err(|e| Status::internal(format!("failed to build resource endpoint: {}", e)))?
      .to_string();

    Ok(Some(mises_proto::AccessEndpoint {
      protocol,
      address,
      expires_at_epoch_seconds: None,
      metadata: HashMap::new(),
    }))
  }

  async fn caller_identity_id<T>(&self, request: &Request<T>) -> Result<Uuid, Status>
  where
    T: Send + Sync,
  {
    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());
    let claims = extract_optional_claims(request, identity_service)
      .await?
      .or_unauthenticated("Missing authorization")?;

    Self::identity_from_claims(&claims)
  }

  async fn map_accessible_resource(
    &self,
    resource: mises_core::service::resource_gateway::AccessibleResource,
  ) -> Result<mises_proto::AccessibleResource, Status> {
    let mut endpoints = Vec::new();
    if let Some(endpoint) = self.endpoint_for_resource(resource.resource_id, &resource.resource_type)? {
      endpoints.push(endpoint);
    }

    Ok(mises_proto::AccessibleResource {
      resource_id: resource.resource_id.to_string(),
      resource_type: resource.resource_type,
      permissions: resource.permissions,
      endpoints,
    })
  }
}

#[tonic::async_trait]
impl<R> mises_proto::resource_gateway_service_server::ResourceGatewayService
  for ResourceGatewayService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  async fn list_accessible_resources(
    &self,
    request: Request<mises_proto::ListAccessibleResourcesRequest>,
  ) -> Result<Response<mises_proto::ListAccessibleResourcesResponse>, Status> {
    let caller_id = self.caller_identity_id(&request).await?;
    let filter = request.into_inner().resource_type.and_then(|value| {
      let trimmed = value.trim().to_string();
      if trimmed.is_empty() {
        None
      } else {
        Some(trimmed)
      }
    });

    let core_service =
      mises_core::service::resource_gateway::ResourceGatewayService::new(self.repo.clone());
    let resources = core_service
      .list_accessible_resources(caller_id, filter.as_deref())
      .await
      .or_internal("failed to query accessible resources")?;

    let mut proto_resources = Vec::new();
    for resource in resources {
      proto_resources.push(self.map_accessible_resource(resource).await?);
    }

    Ok(Response::new(
      mises_proto::ListAccessibleResourcesResponse {
        resources: proto_resources,
      },
    ))
  }

  async fn get_accessible_resource(
    &self,
    request: Request<mises_proto::GetAccessibleResourceRequest>,
  ) -> Result<Response<mises_proto::AccessibleResource>, Status> {
    let caller_id = self.caller_identity_id(&request).await?;
    let resource_id = parse_uuid(&request.get_ref().resource_id)?;

    let core_service =
      mises_core::service::resource_gateway::ResourceGatewayService::new(self.repo.clone());
    let resource = core_service
      .get_accessible_resource(caller_id, resource_id)
      .await
      .or_internal("failed to query accessible resource")?
      .or_not_found("resource is not accessible")?;

    Ok(Response::new(self.map_accessible_resource(resource).await?))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use mises_graph::{InMemoryKeyValueRepository, UuidGenerator};
  use mises_core::service::resource_gateway::AccessibleResource;
  use url::Url;

  #[tokio::test]
  async fn endpoint_for_file_system_resource_includes_files_suffix() {
    let repo: InMemoryKeyValueRepository<_, _, _, _> = InMemoryKeyValueRepository::new_in_memory(UuidGenerator::new());
    let service = ResourceGatewayService::new(repo, "device".to_string(), Url::parse("http://localhost:8080/").unwrap());

    let resource_id = Uuid::new_v4();
    let accessible = AccessibleResource {
      resource_id,
      resource_type: "file-system".to_string(),
      permissions: vec!["readwrite".to_string()],
    };

    let out = service.map_accessible_resource(accessible).await.unwrap();
    let endpoint = out.endpoints.first().expect("expected one endpoint");
    assert!(endpoint.address.contains(&format!("/resources/{}/files", resource_id)));
  }

  #[tokio::test]
  async fn endpoint_for_non_file_system_resource_does_not_include_files_suffix() {
    let repo: InMemoryKeyValueRepository<_, _, _, _> = InMemoryKeyValueRepository::new_in_memory(UuidGenerator::new());
    let service = ResourceGatewayService::new(repo, "device".to_string(), Url::parse("http://localhost:8080/").unwrap());

    let resource_id = Uuid::new_v4();
    let accessible = AccessibleResource {
      resource_id,
      resource_type: "kv".to_string(),
      permissions: vec!["readwrite".to_string()],
    };

    let out = service.map_accessible_resource(accessible).await.unwrap();
    let endpoint = out.endpoints.first().expect("expected one endpoint");
    assert!(!endpoint.address.ends_with("/files"));
  }
}
