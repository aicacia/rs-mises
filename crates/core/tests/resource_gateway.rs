#![cfg(feature = "in-memory")]

use mises_core::{
  model::{
    edge::{EdgeProps, EdgeType},
    identity::IdentityMeta,
    node::{NodeMeta, NodeType},
    resource::ResourceMeta,
  },
  service::resource_gateway::{FileAccessOperation, ResourceGatewayService},
  CoreError,
};
use mises_graph::Executor;

mod common;

use common::{create_identity, create_resource, make_repo};

#[tokio::test]
async fn list_accessible_resources_filters_by_owns_and_type() {
  let repo = make_repo();
  let service = ResourceGatewayService::new(repo.clone());

  let alice_id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "alice".to_string(),
      encrypted_password: "pw".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let bob_id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "bob".to_string(),
      encrypted_password: "pw".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let fs_resource = create_resource(&repo, "file-system").await;
  let kv_resource = create_resource(&repo, "kv").await;

  repo
    .create_edge(
      EdgeType::Owns.as_str().to_string(),
      alice_id,
      fs_resource,
      EdgeProps::Owns {
        since: None,
        until: None,
      },
    )
    .await
    .expect("create owns edge for alice");

  repo
    .create_edge(
      EdgeType::Owns.as_str().to_string(),
      bob_id,
      kv_resource,
      EdgeProps::Owns {
        since: None,
        until: None,
      },
    )
    .await
    .expect("create owns edge for bob");

  let all = service
    .list_accessible_resources(alice_id, None)
    .await
    .expect("list resources for alice");

  assert_eq!(all.len(), 1);
  assert_eq!(all[0].resource_id, fs_resource);
  assert_eq!(all[0].resource_type, "file-system");

  let filtered = service
    .list_accessible_resources(alice_id, Some("file-system"))
    .await
    .expect("list filtered resources for alice");

  assert_eq!(filtered.len(), 1);
  assert_eq!(filtered[0].resource_id, fs_resource);

  let empty = service
    .list_accessible_resources(alice_id, Some("kv"))
    .await
    .expect("list mismatched filtered resources for alice");

  assert!(empty.is_empty());
}

#[tokio::test]
async fn get_accessible_resource_returns_none_for_unowned() {
  let repo = make_repo();
  let service = ResourceGatewayService::new(repo.clone());

  let alice_id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "alice".to_string(),
      encrypted_password: "pw".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let resource_id = create_resource(&repo, "file-system").await;

  let value = service
    .get_accessible_resource(alice_id, resource_id)
    .await
    .expect("get accessible resource");

  assert!(value.is_none());
}

#[tokio::test]
async fn check_file_access_read_and_write_on_readwrite_resource() {
  let repo = make_repo();
  let service = ResourceGatewayService::new(repo.clone());

  let alice_id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "alice".to_string(),
      encrypted_password: "pw".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let resource_id = create_resource(&repo, "file-system").await;

  repo
    .create_edge(
      EdgeType::Owns.as_str().to_string(),
      alice_id,
      resource_id,
      EdgeProps::Owns {
        since: None,
        until: None,
      },
    )
    .await
    .expect("create owns edge for alice");

  assert!(
    service
      .check_file_access(alice_id, resource_id, FileAccessOperation::Read)
      .await
      .is_ok()
  );
  assert!(
    service
      .check_file_access(alice_id, resource_id, FileAccessOperation::Write)
      .await
      .is_ok()
  );
}

#[tokio::test]
async fn check_file_access_write_denied_for_readonly_resource() {
  let repo = make_repo();
  let service = ResourceGatewayService::new(repo.clone());

  let alice_id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "alice".to_string(),
      encrypted_password: "pw".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let resource_id = repo
    .create_node(
      NodeType::Resource.as_str().to_string(),
      NodeMeta::Resource(ResourceMeta {
        r#type: "file-system".to_string(),
        permissions: vec!["readonly".to_string()],
      }),
    )
    .await
    .unwrap()
    .id;

  repo
    .create_edge(
      EdgeType::Owns.as_str().to_string(),
      alice_id,
      resource_id,
      EdgeProps::Owns {
        since: None,
        until: None,
      },
    )
    .await
    .expect("create owns edge for alice");

  assert!(
    service
      .check_file_access(alice_id, resource_id, FileAccessOperation::Read)
      .await
      .is_ok()
  );

  let err = service
    .check_file_access(alice_id, resource_id, FileAccessOperation::Write)
    .await
    .unwrap_err();

  assert!(matches!(err, CoreError::Forbidden));
}
