use mises_core::{
  service::graph::{BootstrapOptions, GraphService},
  traits::Repository,
};
use tonic::{Request, Response, Status};

pub struct BootstrapService<R>
where
  R: Repository,
{
  repo: R,
}

impl<R> BootstrapService<R>
where
  R: Repository,
{
  pub fn new(repo: R) -> Self {
    Self { repo }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::bootstrap_service_server::BootstrapService for BootstrapService<R>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  async fn bootstrap(
    &self,
    request: Request<mises_proto::BootstrapRequest>,
  ) -> Result<Response<mises_proto::BootstrapResponse>, Status> {
    let graph_service = GraphService::new(self.repo.clone());

    let bootstrap_result = graph_service
      .bootstrap(BootstrapOptions {
        root_group_name: Some(request.get_ref().root_group_name.clone()),
        owner_name: Some(request.get_ref().owner_name.clone()),
        device_name: Some(request.get_ref().device_name.clone()),
        now: Some(chrono::Utc::now()),
        test_seed: None,
      })
      .await
      .map_err(|e| Status::internal(e.to_string()))?;

    let reply = mises_proto::BootstrapResponse {
      root_group: bootstrap_result.root_group.to_string(),
      master_key_created: bootstrap_result.master_key_created,
      master_key_public_key: bootstrap_result.master_key_public_key,
      owner_user: bootstrap_result.owner_user.to_string(),
      device: bootstrap_result.device.to_string(),
    };

    Ok(Response::new(reply))
  }
}
