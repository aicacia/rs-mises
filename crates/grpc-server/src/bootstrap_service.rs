use mises_core::{
  service::graph::{BootstrapOptions, GraphService},
  traits::Repository,
};
use tonic::{Request, Response, Status};

pub struct BootstrapService<R>
where
  R: Repository,
{
  graph_service: GraphService<R>,
}

impl<R> BootstrapService<R>
where
  R: Repository,
{
  pub fn new(graph_service: GraphService<R>) -> Self {
    Self { graph_service }
  }
}

#[tonic::async_trait]
impl<R> mises_proto::bootstrap_service_server::BootstrapService for BootstrapService<R>
where
  R: Repository + Send + Sync + 'static,
{
  async fn bootstrap(
    &self,
    request: Request<mises_proto::BootstrapRequest>,
  ) -> Result<Response<mises_proto::BootstrapResponse>, Status> {
    let bootstrap_result = self
      .graph_service
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
