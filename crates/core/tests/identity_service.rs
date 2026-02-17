#![cfg(feature = "in-memory")]

use mises_core::{
  model::identity::{IdentityMeta, IdentityType},
  service::identity::IdentityService,
};
use uuid::Uuid;

mod common;

use common::{create_identity, make_repo};

#[tokio::test]
async fn get_node_by_id_and_identity_type_happy_path() {
  let repo = make_repo();
  let service = IdentityService::new(repo.clone(), "test-device".to_string());

  let id = create_identity(
    &repo,
    IdentityMeta::Application {
      name: "app".to_string(),
      oidc: Box::new(None),
    },
  )
  .await;

  let node = service
    .get_node_by_id_and_identity_type(id, IdentityType::Application)
    .await
    .expect("should find application node");

  assert_eq!(node.id, id);
}

#[tokio::test]
async fn get_node_by_id_and_identity_type_mismatch_and_not_found() {
  let repo = make_repo();
  let service = IdentityService::new(repo.clone(), "test-device".to_string());

  let id = create_identity(
    &repo,
    IdentityMeta::User {
      name: "user".to_string(),
      encrypted_password: "pass".to_string(),
      force_password_reset: None,
    },
  )
  .await;

  let res = service
    .get_node_by_id_and_identity_type(id, IdentityType::Application)
    .await;
  match res {
    Err(e) => match e {
      mises_core::CoreError::InvalidInput(_) => {}
      _ => panic!("expected InvalidInput for mismatched type"),
    },
    Ok(_) => panic!("expected error for mismatched type"),
  }

  let missing = Uuid::new_v4();
  let res = service
    .get_node_by_id_and_identity_type(missing, IdentityType::Application)
    .await;
  match res {
    Err(e) => match e {
      mises_core::CoreError::NotFound => {}
      _ => panic!("expected NotFound for missing node"),
    },
    Ok(_) => panic!("expected error for missing node"),
  }
}
