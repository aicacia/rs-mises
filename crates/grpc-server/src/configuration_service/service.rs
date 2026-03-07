use tonic::{Request, Response, Status};
use url::Url;

use mises_core::{service::identity::IdentityService, traits::Repository};

pub struct ConfigurationService<R>
where
  R: Repository,
{
  repo: R,
  device_id: String,
  issuer: String,
  public_uri: Url,
}

impl<R> ConfigurationService<R>
where
  R: Repository,
{
  pub fn new(repo: R, device_id: String, issuer: String, public_uri: Url) -> Self {
    Self {
      repo,
      device_id,
      issuer,
      public_uri,
    }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::configuration_service_server::ConfigurationService for ConfigurationService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  async fn get(&self, _: Request<()>) -> Result<Response<mises_proto::Configuration>, Status> {
    let identity_service = IdentityService::new(self.repo.clone(), self.device_id.clone());

    let service = match identity_service
      .find_service_by_name("mises")
      .await
      .map_err(|e| Status::internal(format!("failed to find mises service: {}", e)))?
    {
      Some(service) => service,
      None => return Err(Status::not_found("mises service not found")),
    };

    let issuer = self.issuer.clone();
    let device_id = self.device_id.clone();
    let service_id = service.id.to_string();
    // TODO: we need an actual client_id here, but for now we can just use the service_id as a placeholder
    // we will need to implement client registration in the future to get a real client_id
    let client_id = service_id.clone();
    let public_uri = self.public_uri.to_string();

    Ok(Response::new(mises_proto::Configuration {
      issuer,
      device_id,
      service_id,
      client_id,
      public_uri,
    }))
  }
}
