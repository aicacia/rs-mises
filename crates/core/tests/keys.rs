#![cfg(feature = "in-memory")]

use mises_core::service::identity::IdentityService;
use mises_graph::Executor;

mod common;

use common::make_repo;

#[tokio::test]
async fn user_jwt_key_creation_and_retrieval() {
  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (_user_node, key_node) = identity_service
    .create_user("testuser".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user with key");

  let key_id = key_node.id;
  let key_meta = match &key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  assert!(
    !key_meta.public_key.is_empty(),
    "public key should be present"
  );
  assert!(
    key_meta.private_key.is_some(),
    "private key should be present"
  );
  assert!(
    !key_meta.derivation_path.is_empty(),
    "derivation path should be present"
  );

  let kid_uuid = uuid::Uuid::parse_str(&key_id.to_string()).expect("parse key id");
  let retrieved_key_node = repo
    .get_node_by_id(kid_uuid)
    .await
    .expect("should find key in repo")
    .expect("key should exist");

  let retrieved_key_meta = match &retrieved_key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  assert_eq!(
    key_meta.public_key, retrieved_key_meta.public_key,
    "public keys should match"
  );
  assert_eq!(
    key_meta.derivation_path, retrieved_key_meta.derivation_path,
    "derivation paths should match"
  );
  assert_eq!(
    key_meta.private_key, retrieved_key_meta.private_key,
    "private keys should match"
  );

  let _encoding = key_meta
    .jwt_encoding_key()
    .expect("encoding key should be derivable");
  let _decoding = key_meta
    .jwt_decoding_key()
    .expect("decoding key should be derivable");
}

#[tokio::test]
async fn application_jwt_key_creation_and_retrieval() {
  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (_app_node, key_node) = identity_service
    .create_application("test-app".to_string(), None)
    .await
    .expect("create app with key");

  let key_id = key_node.id;
  let key_meta = match &key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  assert!(
    !key_meta.public_key.is_empty(),
    "public key should be present"
  );
  assert!(
    key_meta.private_key.is_some(),
    "private key should be present"
  );
  assert!(
    !key_meta.derivation_path.is_empty(),
    "derivation path should be present"
  );

  let kid_uuid = uuid::Uuid::parse_str(&key_id.to_string()).expect("parse key id");
  let retrieved_key_node = repo
    .get_node_by_id(kid_uuid)
    .await
    .expect("should find key in repo")
    .expect("key should exist");

  let retrieved_key_meta = match &retrieved_key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  assert_eq!(
    key_meta.public_key, retrieved_key_meta.public_key,
    "public keys should match"
  );
  assert_eq!(
    key_meta.derivation_path, retrieved_key_meta.derivation_path,
    "derivation paths should match"
  );
  assert_eq!(
    key_meta.private_key, retrieved_key_meta.private_key,
    "private keys should match"
  );

  let _encoding = key_meta
    .jwt_encoding_key()
    .expect("encoding key should be derivable");
  let _decoding = key_meta
    .jwt_decoding_key()
    .expect("decoding key should be derivable");
}

#[tokio::test]
async fn jwt_signature_verification_fails_with_wrong_key() {
  use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
  use serde::{Deserialize, Serialize};

  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (user1, key_node1) = identity_service
    .create_user("user1".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user1");

  let (_user2, key_node2) = identity_service
    .create_user("user2".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user2");

  let key_meta1 = match &key_node1.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  let key_meta2 = match &key_node2.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  #[derive(Debug, Serialize, Deserialize)]
  struct TestClaims {
    sub: String,
    iat: i64,
    exp: i64,
  }

  let encoding1 = key_meta1.jwt_encoding_key().expect("encoding key1");
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;
  let claims1 = TestClaims {
    sub: user1.id.to_string(),
    iat: now,
    exp: now + 3600,
  };

  let mut header = Header::new(Algorithm::EdDSA);
  header.kid = Some(key_node1.id.to_string());
  let token_signed_by_user1 =
    encode(&header, &claims1, &encoding1).expect("token encoding by user1");

  let decoding2 = key_meta2.jwt_decoding_key().expect("decoding key2");
  let validation = Validation::new(Algorithm::EdDSA);

  let result = decode::<TestClaims>(&token_signed_by_user1, &decoding2, &validation);
  assert!(
    result.is_err(),
    "verification should fail when using a different key"
  );
}

#[tokio::test]
async fn jwt_verification_rejects_expired_token() {
  use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
  use serde::{Deserialize, Serialize};

  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (user_node, key_node) = identity_service
    .create_user("testuser".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user with key");

  let key_meta = match &key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  #[derive(Debug, Serialize, Deserialize)]
  struct TestClaims {
    sub: String,
    iat: i64,
    exp: i64,
  }

  let encoding = key_meta.jwt_encoding_key().expect("encoding key");
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;
  let claims = TestClaims {
    sub: user_node.id.to_string(),
    iat: now - 7200,
    exp: now - 3600,
  };

  let mut header = Header::new(Algorithm::EdDSA);
  header.kid = Some(key_node.id.to_string());
  let token = encode(&header, &claims, &encoding).expect("token encoding");

  let kid_uuid = uuid::Uuid::parse_str(&key_node.id.to_string()).expect("parse key id");
  let retrieved_key_node = repo
    .get_node_by_id(kid_uuid)
    .await
    .expect("should find key in repo")
    .expect("key should exist");

  let retrieved_key_meta = match &retrieved_key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  let decoding = retrieved_key_meta.jwt_decoding_key().expect("decoding key");
  let validation = Validation::new(Algorithm::EdDSA);

  let result = decode::<TestClaims>(&token, &decoding, &validation);
  assert!(
    result.is_err(),
    "verification should fail for expired token"
  );
}

#[tokio::test]
async fn different_users_have_different_keys() {
  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (_user1, key_node1) = identity_service
    .create_user("user1".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user1");

  let (_user2, key_node2) = identity_service
    .create_user("user2".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user2");

  let key_meta1 = match &key_node1.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  let key_meta2 = match &key_node2.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  assert_ne!(
    key_meta1.public_key, key_meta2.public_key,
    "different users must have different public keys (derived from different paths)"
  );
  assert_ne!(
    key_meta1.derivation_path, key_meta2.derivation_path,
    "different users must have different derivation paths"
  );
}

#[tokio::test]
async fn jwt_token_can_be_verified_with_same_key() {
  use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
  use serde::{Deserialize, Serialize};

  let repo = make_repo();
  let graph_service = mises_core::service::graph::GraphService::new(repo.clone());

  let opts = mises_core::service::graph::BootstrapOptions::builder("test-device")
    .root_group_name("Everything")
    .build();
  graph_service.bootstrap(opts).await.expect("bootstrap");

  let identity_service = IdentityService::new(repo.clone(), "test-device".to_string());

  let (user_node, key_node) = identity_service
    .create_user("testuser".to_string(), "hashed".to_string(), None)
    .await
    .expect("create user with key");

  let key_meta = match &key_node.metadata {
    mises_core::model::node::NodeMeta::Key(km) => km.clone(),
    _ => panic!("expected key metadata"),
  };

  #[derive(Debug, Serialize, Deserialize)]
  struct TestClaims {
    sub: String,
    iat: i64,
    exp: i64,
  }

  let encoding = key_meta.jwt_encoding_key().expect("encoding key");
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;
  let claims = TestClaims {
    sub: user_node.id.to_string(),
    iat: now,
    exp: now + 3600,
  };

  let mut header = Header::new(Algorithm::EdDSA);
  header.kid = Some(key_node.id.to_string());
  let token = encode(&header, &claims, &encoding).expect("token encoding");

  let decoding = key_meta.jwt_decoding_key().expect("decoding key");
  let validation = Validation::new(Algorithm::EdDSA);

  let result = decode::<TestClaims>(&token, &decoding, &validation).expect("token should verify");
  assert_eq!(result.claims.sub, user_node.id.to_string());
}
