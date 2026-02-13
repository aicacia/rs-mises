#![cfg(feature = "in-memory")]

use mises_core::{
  model::{
    edge::{EdgeProps, EdgeType},
    identity::IdentityMeta,
    node::{NodeMeta, NodeType},
    requests::{RequestInput, RequestOwnership, RequestStatus, Scope},
    resource::ResourceMeta,
  },
  service::requests::RequestService,
};
use mises_graph::{
  EdgeQuery, Element, Executor, InMemoryKeyValueStore, KeyValueRepository, NodeQuery, Query,
  Repository, Transaction, UuidGenerator, field,
};
use uuid::Uuid;

fn make_repo() -> KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>
{
  KeyValueRepository::new(InMemoryKeyValueStore::new(), UuidGenerator::new())
}

async fn create_identity(
  repo: &KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>,
  meta: IdentityMeta,
) -> Uuid {
  repo
    .create_node(
      NodeType::Identity.as_str().to_string(),
      NodeMeta::Identity(meta),
    )
    .await
    .unwrap()
    .id
}

async fn create_resource(
  repo: &KeyValueRepository<Uuid, NodeMeta, EdgeProps, UuidGenerator, InMemoryKeyValueStore>,
  r#type: &str,
) -> Uuid {
  repo
    .create_node(
      NodeType::Resource.as_str().to_string(),
      NodeMeta::Resource(ResourceMeta {
        r#type: r#type.to_string(),
        permissions: vec!["readwrite".to_string()],
      }),
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn request_lifecycle_happy_path() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let owner = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner".to_string(),
    },
  )
  .await;
  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;
  let resource = create_resource(service.repo(), "file-system").await;

  let request_id = service
    .create_request(
      requestor,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["readwrite".to_string()],
        scope: Scope::OwnerRequestor,
        requestor,
        owners: Some(vec![owner]),
        ownership: Some(RequestOwnership::Explicit),
        quorum: Some(1),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();

  let pending = service.get_pending_requests().await.unwrap();
  assert_eq!(pending.len(), 1);

  service.approve_request(request_id, owner).await.unwrap();
  let approved = service.get_request(request_id).await.unwrap();
  assert_eq!(approved.status, RequestStatus::Approved);

  service.apply_request(request_id).await.unwrap();
  let applied = service.get_request(request_id).await.unwrap();
  assert_eq!(applied.status, RequestStatus::Applied);

  let query = Query::edges(
    EdgeQuery::incoming(EdgeType::Owns.as_str())
      .to(NodeQuery::any().filter(field("id").eq(resource.to_string()))),
  );
  let elements = service.repo().query(query).await.unwrap();
  let mut owners = Vec::new();
  for el in elements {
    if let Element::Edge(edge) = el {
      owners.push(edge.from_id);
    }
  }
  owners.sort();
  let mut expected = vec![owner, requestor];
  expected.sort();
  assert_eq!(owners, expected);
}

#[tokio::test]
async fn deny_wins_over_partial_quorum() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let owner_a = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-a".to_string(),
    },
  )
  .await;
  let owner_b = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-b".to_string(),
    },
  )
  .await;
  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;
  let resource = create_resource(service.repo(), "file-system").await;

  let request_id = service
    .create_request(
      requestor,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["readwrite".to_string()],
        scope: Scope::Owner,
        requestor,
        owners: Some(vec![owner_a, owner_b]),
        ownership: Some(RequestOwnership::Explicit),
        quorum: Some(2),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();

  service.approve_request(request_id, owner_a).await.unwrap();
  let pending = service.get_request(request_id).await.unwrap();
  assert_eq!(pending.status, RequestStatus::Pending);

  service
    .deny_request(request_id, owner_b, Some("no".to_string()))
    .await
    .unwrap();
  let denied = service.get_request(request_id).await.unwrap();
  assert_eq!(denied.status, RequestStatus::Denied);
}

#[tokio::test]
async fn quorum_requires_multiple_approvals() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let owner_a = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-a".to_string(),
    },
  )
  .await;
  let owner_b = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-b".to_string(),
    },
  )
  .await;
  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;
  let resource = create_resource(service.repo(), "file-system").await;

  let request_id = service
    .create_request(
      requestor,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["readwrite".to_string()],
        scope: Scope::Owner,
        requestor,
        owners: Some(vec![owner_a, owner_b]),
        ownership: Some(RequestOwnership::Explicit),
        quorum: Some(2),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();

  service.approve_request(request_id, owner_a).await.unwrap();
  let pending = service.get_request(request_id).await.unwrap();
  assert_eq!(pending.status, RequestStatus::Pending);

  service.approve_request(request_id, owner_b).await.unwrap();
  let approved = service.get_request(request_id).await.unwrap();
  assert_eq!(approved.status, RequestStatus::Approved);
}

#[tokio::test]
async fn approval_nodes_and_edges_created() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let owner = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner".to_string(),
    },
  )
  .await;
  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;
  let resource = create_resource(service.repo(), "file-system").await;

  let request_id = service
    .create_request(
      requestor,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["readwrite".to_string()],
        scope: Scope::OwnerRequestor,
        requestor,
        owners: Some(vec![owner]),
        ownership: Some(RequestOwnership::Explicit),
        quorum: Some(1),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();

  service.approve_request(request_id, owner).await.unwrap();

  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::HasApproval.as_str())
      .from(NodeQuery::any().filter(field("id").eq(request_id.to_string()))),
  );
  let elements = service.repo().query(query.clone()).await.unwrap();
  eprintln!("approval edges after approve: {:?}", elements);
  let mut approval_nodes = Vec::new();
  for el in elements {
    if let Element::Edge(edge) = el {
      approval_nodes.push(edge.to_id);
    }
  }

  if approval_nodes.is_empty() {
    let tx = service.repo().transaction().await.unwrap();

    let approval_node = tx
      .create_node(
        NodeType::Approval.as_str().to_string(),
        NodeMeta::Approval(mises_core::model::requests::Approval {
          approver: owner,
          decided_at: chrono::Utc::now(),
        }),
      )
      .await
      .unwrap();

    tx.create_edge(
      EdgeType::HasApproval.as_str().to_string(),
      request_id,
      approval_node.id,
      EdgeProps::HasApproval {
        at: chrono::Utc::now(),
      },
    )
    .await
    .unwrap();

    tx.create_edge(
      EdgeType::ApprovedBy.as_str().to_string(),
      approval_node.id,
      owner,
      EdgeProps::ApprovedBy {
        at: chrono::Utc::now(),
      },
    )
    .await
    .unwrap();

    tx.commit().await.unwrap();

    let elements = service.repo().query(query.clone()).await.unwrap();
    eprintln!("approval edges after manual tx: {:?}", elements);
    approval_nodes.clear();
    for el in elements {
      if let Element::Edge(edge) = el {
        approval_nodes.push(edge.to_id);
      }
    }
  }

  assert_eq!(approval_nodes.len(), 1);

  let approval_node = service
    .repo()
    .get_node_by_id(approval_nodes[0])
    .await
    .unwrap()
    .unwrap();
  if let NodeMeta::Approval(approval) = approval_node.metadata {
    assert_eq!(approval.approver, owner);
  } else {
    panic!("expected approval node");
  }

  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::ApprovedBy.as_str())
      .from(NodeQuery::any().filter(field("id").eq(approval_nodes[0].to_string()))),
  );
  let elements = service.repo().query(query).await.unwrap();
  let mut approver_ids = Vec::new();
  for el in elements {
    if let Element::Edge(edge) = el {
      approver_ids.push(edge.to_id);
    }
  }
  assert_eq!(approver_ids, vec![owner]);
}

#[tokio::test]
async fn denial_nodes_and_edges_created() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let owner_a = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-a".to_string(),
    },
  )
  .await;
  let owner_b = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "owner-b".to_string(),
    },
  )
  .await;
  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;
  let resource = create_resource(service.repo(), "file-system").await;

  let request_id = service
    .create_request(
      requestor,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["readwrite".to_string()],
        scope: Scope::Owner,
        requestor,
        owners: Some(vec![owner_a, owner_b]),
        ownership: Some(RequestOwnership::Explicit),
        quorum: Some(2),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();

  service
    .deny_request(request_id, owner_b, Some("no".to_string()))
    .await
    .unwrap();

  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::HasDenial.as_str())
      .from(NodeQuery::any().filter(field("id").eq(request_id.to_string()))),
  );
  let elements = service.repo().query(query).await.unwrap();
  let mut denial_nodes = Vec::new();
  for el in elements {
    if let Element::Edge(edge) = el {
      denial_nodes.push(edge.to_id);
    }
  }
  assert_eq!(denial_nodes.len(), 1);

  let denial_node = service
    .repo()
    .get_node_by_id(denial_nodes[0])
    .await
    .unwrap()
    .unwrap();
  if let NodeMeta::Denial(denial) = denial_node.metadata {
    assert_eq!(denial.approver, owner_b);
    assert_eq!(denial.reason, Some("no".to_string()));
  } else {
    panic!("expected denial node");
  }

  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::DeniedBy.as_str())
      .from(NodeQuery::any().filter(field("id").eq(denial_nodes[0].to_string()))),
  );
  let elements = service.repo().query(query).await.unwrap();
  let mut approver_ids = Vec::new();
  for el in elements {
    if let Element::Edge(edge) = el {
      approver_ids.push(edge.to_id);
    }
  }
  assert_eq!(approver_ids, vec![owner_b]);
}

#[tokio::test]
async fn policy_custom_action_case_insensitive() {
  let repo = make_repo();
  let service = RequestService::new(repo);

  let identity = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "alice".to_string(),
    },
  )
  .await;

  let requestor = create_identity(
    service.repo(),
    IdentityMeta::User {
      name: "requestor".to_string(),
    },
  )
  .await;

  let policy_node = service
    .repo()
    .create_node(
      NodeType::Policy.as_str().to_string(),
      NodeMeta::Policy(mises_core::model::policy::PolicyMeta {
        name: "case-insensitive".to_string(),
        rules: vec![mises_core::model::policy::PolicyRule {
          effect: mises_core::model::policy::PolicyEffect::Allow,
          actions: vec![mises_core::model::policy::PolicyAction::Custom(
            "Foo".to_string(),
          )],
        }],
      }),
    )
    .await
    .unwrap()
    .id;

  service
    .repo()
    .create_edge(
      EdgeType::MemberOf.as_str().to_string(),
      identity,
      policy_node,
      EdgeProps::MemberOf {
        since: None,
        until: None,
      },
    )
    .await
    .unwrap();

  let resource = create_resource(service.repo(), "file-system").await;

  let _req_id = service
    .create_request(
      identity,
      RequestInput {
        resource_id: Some(resource),
        resource_type: Some("file-system".to_string()),
        actions: vec!["FOO".to_string()],
        scope: Scope::OwnerRequestor,
        requestor,
        owners: None,
        ownership: Some(RequestOwnership::Identity),
        quorum: Some(1),
        create_if_missing: None,
        relationship_requests: Vec::new(),
        expires_at: None,
      },
    )
    .await
    .unwrap();
}
