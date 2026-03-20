use alloc::{
  collections::BTreeSet,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use mises_graph::{EdgeQuery, Element, Executor, NodeQuery, Query, Transaction, field};

use crate::{
  CoreError, InvalidInput, Result,
  model::{
    edge::{EdgeProps, EdgeType},
    node::{NodeMeta, NodeType},
    policy::{PolicyAction, PolicyEffect, PolicyMeta},
    requests::{Approval, Denial, Request, RequestInput, RequestOwnership, RequestStatus, Scope},
    resource::ResourceMeta,
  },
  traits::Repository,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
  Allowed,
  Denied,
  NotApplicable,
}

#[derive(Clone)]
pub struct RequestService<E>
where
  E: Repository,
{
  exec: E,
}

impl<E> RequestService<E>
where
  E: Repository,
{
  pub fn new(exec: E) -> Self {
    Self { exec }
  }

  pub fn exec(&self) -> &E {
    &self.exec
  }

  pub async fn create_request(&self, requested_for: Uuid, input: RequestInput) -> Result<Uuid> {
    self.ensure_identity_exists(requested_for).await?;
    self.ensure_identity_exists(input.requestor).await?;

    if let Some(owners) = &input.owners {
      for owner in owners {
        self.ensure_identity_exists(*owner).await?;
      }
    }

    if input.actions.is_empty() && input.relationship_requests.is_empty() {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "request must include actions or relationship requests".to_string(),
      )));
    }

    if input.resource_id.is_none()
      && input.relationship_requests.is_empty()
      && !input.create_if_missing.unwrap_or(false)
    {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "request must include a resource or relationship requests".to_string(),
      )));
    }

    let RequestInput {
      resource_id,
      resource_type,
      actions,
      scope,
      requestor,
      owners,
      ownership,
      quorum,
      create_if_missing,
      relationship_requests,
      expires_at,
    } = input;

    let ownership = ownership.unwrap_or_default();
    if matches!(ownership, RequestOwnership::Explicit)
      && owners.as_ref().is_none_or(|owners| owners.is_empty())
    {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "explicit ownership requires owners".to_string(),
      )));
    }

    let request_input = RequestInput {
      resource_id,
      resource_type,
      actions,
      scope,
      requestor,
      owners,
      ownership: Some(ownership.clone()),
      quorum,
      create_if_missing,
      relationship_requests,
      expires_at,
    };

    let policy_decision = self
      .evaluate_request_policies(requested_for, &request_input.actions)
      .await?;
    if matches!(policy_decision, PolicyDecision::Denied) {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "request denied by policy".to_string(),
      )));
    }

    let (_, quorum) = self
      .resolve_approvers(&request_input, ownership.clone())
      .await?;

    let now = Utc::now();
    let create_if_missing = request_input.create_if_missing.unwrap_or(true);
    let relationship_requests_for_edges = request_input.relationship_requests.clone();

    let request = Request {
      resource_id: request_input.resource_id,
      resource_type: request_input.resource_type,
      actions: request_input.actions,
      scope: request_input.scope,
      requestor: request_input.requestor,
      requested_for: Some(requested_for),
      owners: request_input.owners,
      create_if_missing,
      ownership,
      relationship_requests: request_input.relationship_requests,
      status: RequestStatus::Pending,
      quorum,
      created_at: now,
      applied_at: None,
      expires_at: request_input.expires_at,
    };

    let tx = self.exec.transaction().await?;
    let node = tx
      .create_node(
        NodeType::Request.as_str().to_string(),
        NodeMeta::Request(request),
      )
      .await?;

    tx.create_edge(
      EdgeType::RequestedFor.as_str().to_string(),
      node.id,
      requested_for,
      EdgeProps::RequestedFor { at: now },
    )
    .await?;

    if let Some(resource_id) = request_input.resource_id {
      tx.create_edge(
        EdgeType::AppliesTo.as_str().to_string(),
        node.id,
        resource_id,
        EdgeProps::AppliesTo { at: now },
      )
      .await?;
    }

    for relationship in &relationship_requests_for_edges {
      tx.create_edge(
        EdgeType::AppliesTo.as_str().to_string(),
        node.id,
        relationship.object,
        EdgeProps::AppliesTo { at: now },
      )
      .await?;
    }

    tx.commit().await?;

    Ok(node.id)
  }

  pub async fn get_request(&self, id: Uuid) -> Result<Request> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match node.metadata {
      NodeMeta::Request(request) => Ok(request),
      _ => Err(CoreError::NotFound),
    }
  }

  pub async fn get_pending_requests(&self) -> Result<Vec<Request>> {
    let query = Query::nodes(
      NodeQuery::new(NodeType::Request.as_str())
        .filter(field("metadata.status").eq(RequestStatus::Pending.as_str())),
    );

    let elements = self.exec.query(query).await?;
    let mut results = Vec::new();

    for el in elements {
      if let Element::Node(node) = el
        && let NodeMeta::Request(request) = node.metadata
      {
        results.push(request);
      }
    }

    Ok(results)
  }

  pub async fn approve_request(&self, id: Uuid, approver_id: Uuid) -> Result<()> {
    self.ensure_identity_exists(approver_id).await?;

    let mut node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    let mut request = match node.metadata {
      NodeMeta::Request(request) => request,
      _ => return Err(CoreError::NotFound),
    };

    if matches!(
      request.status,
      RequestStatus::Denied | RequestStatus::Applied
    ) {
      return Err(CoreError::Conflict);
    }

    let eligible_approvers = self.eligible_approvers_for_request(&request).await?;
    if !eligible_approvers.contains(&approver_id) {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "approver is not eligible".to_string(),
      )));
    }

    let check_query = Query::edges(
      EdgeQuery::outgoing(EdgeType::HasApproval.as_str())
        .from(NodeQuery::any().filter(field("id").eq(node.id.to_string()))),
    );

    let elements = self.exec.query(check_query).await?;
    let mut existing_approval_count = 0usize;
    for el in elements {
      if let Element::Edge(edge) = el
        && let Some(approval_node) = self.exec.get_node_by_id(edge.to_id).await?
        && let NodeMeta::Approval(approval) = approval_node.metadata
      {
        if approval.approver == approver_id {
          return Ok(());
        }
        existing_approval_count += 1;
      }
    }

    let now = Utc::now();

    let tx = self.exec.transaction().await?;

    let approval_node = tx
      .create_node(
        NodeType::Approval.as_str().to_string(),
        NodeMeta::Approval(Approval {
          approver: approver_id,
          decided_at: now,
        }),
      )
      .await?;

    tx.create_edge(
      EdgeType::HasApproval.as_str().to_string(),
      node.id,
      approval_node.id,
      EdgeProps::HasApproval { at: now },
    )
    .await?;

    tx.create_edge(
      EdgeType::ApprovedBy.as_str().to_string(),
      approval_node.id,
      approver_id,
      EdgeProps::ApprovedBy { at: now },
    )
    .await?;

    let approval_count = existing_approval_count + 1;

    if approval_count >= request.quorum {
      request.status = RequestStatus::Approved;
    }

    node.metadata = NodeMeta::Request(request);

    tx.update_node(node.id, node.metadata, None).await?;
    tx.commit().await?;

    Ok(())
  }

  pub async fn deny_request(
    &self,
    id: Uuid,
    approver_id: Uuid,
    reason: Option<String>,
  ) -> Result<()> {
    self.ensure_identity_exists(approver_id).await?;

    let mut node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    let mut request = match node.metadata {
      NodeMeta::Request(request) => request,
      _ => return Err(CoreError::NotFound),
    };

    if matches!(request.status, RequestStatus::Applied) {
      return Err(CoreError::Conflict);
    }

    let eligible_approvers = self.eligible_approvers_for_request(&request).await?;
    if !eligible_approvers.contains(&approver_id) {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "approver is not eligible".to_string(),
      )));
    }

    let now = Utc::now();

    let tx = self.exec.transaction().await?;

    let denial_node = tx
      .create_node(
        NodeType::Denial.as_str().to_string(),
        NodeMeta::Denial(Denial {
          approver: approver_id,
          decided_at: now,
          reason: reason.clone(),
        }),
      )
      .await?;

    tx.create_edge(
      EdgeType::HasDenial.as_str().to_string(),
      node.id,
      denial_node.id,
      EdgeProps::HasDenial { at: now },
    )
    .await?;

    tx.create_edge(
      EdgeType::DeniedBy.as_str().to_string(),
      denial_node.id,
      approver_id,
      EdgeProps::DeniedBy { at: now, reason },
    )
    .await?;

    request.status = RequestStatus::Denied;

    node.metadata = NodeMeta::Request(request);

    tx.update_node(node.id, node.metadata, None).await?;
    tx.commit().await?;

    Ok(())
  }

  pub async fn apply_request(&self, id: Uuid) -> Result<()> {
    let mut node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    let mut request = match node.metadata {
      NodeMeta::Request(request) => request,
      _ => return Err(CoreError::NotFound),
    };

    if !matches!(request.status, RequestStatus::Approved) {
      return Err(CoreError::Conflict);
    }

    let now = Utc::now();
    let tx = self.exec.transaction().await?;

    let mut resource_id = request.resource_id;
    let mut created_resource = false;
    if resource_id.is_none() && request.create_if_missing {
      let resource_type = request.resource_type.clone().ok_or_else(|| {
        CoreError::InvalidInput(InvalidInput::Other(
          "resource_type is required to create a resource".to_string(),
        ))
      })?;

      let resource_node = tx
        .create_node(
          NodeType::Resource.as_str().to_string(),
          NodeMeta::Resource(ResourceMeta {
            r#type: resource_type,
            permissions: request.actions.clone(),
          }),
        )
        .await?;

      resource_id = Some(resource_node.id);
      request.resource_id = resource_id;
      created_resource = true;

      tx.create_edge(
        EdgeType::AppliesTo.as_str().to_string(),
        node.id,
        resource_node.id,
        EdgeProps::AppliesTo { at: now },
      )
      .await?;
    }

    if let Some(resource_id) = resource_id {
      if !created_resource {
        self.ensure_resource_exists(resource_id).await?;
      }

      let owners = self.resolve_owners_for_apply(&request, resource_id).await?;
      let requestor = request.requestor;

      let mut create_owns_for = Vec::new();
      match request.scope {
        Scope::Owner => create_owns_for.extend(owners),
        Scope::Requestor => create_owns_for.push(requestor),
        Scope::OwnerRequestor => {
          create_owns_for.extend(owners);
          create_owns_for.push(requestor);
        }
      }

      let mut unique = BTreeSet::new();
      for owner_id in create_owns_for {
        if unique.insert(owner_id) {
          tx.create_edge(
            EdgeType::Owns.as_str().to_string(),
            owner_id,
            resource_id,
            EdgeProps::Owns {
              since: Some(now),
              until: None,
            },
          )
          .await?;
        }
      }
    }

    for relationship in &request.relationship_requests {
      let props = Self::edge_props_for_type(relationship.relationship_type.clone(), now)?;
      tx.create_edge(
        relationship.relationship_type.as_str().to_string(),
        relationship.subject,
        relationship.object,
        props,
      )
      .await?;
    }

    request.status = RequestStatus::Applied;
    request.applied_at = Some(now);
    node.metadata = NodeMeta::Request(request);

    tx.update_node(node.id, node.metadata, None).await?;
    tx.commit().await?;

    Ok(())
  }

  async fn resolve_approvers(
    &self,
    input: &RequestInput,
    ownership: RequestOwnership,
  ) -> Result<(Vec<Uuid>, usize)> {
    let mut approvers = BTreeSet::new();

    match ownership {
      RequestOwnership::Requestor => {
        approvers.insert(input.requestor);
      }
      RequestOwnership::Explicit => {
        if let Some(owners) = &input.owners {
          for owner in owners {
            approvers.insert(*owner);
          }
        }
      }
      RequestOwnership::Identity => {
        if let Some(resource_id) = input.resource_id {
          for owner in self.owners_of(resource_id).await? {
            approvers.insert(owner);
          }
        }

        if approvers.is_empty() {
          for relationship in &input.relationship_requests {
            for owner in self.owners_of(relationship.object).await? {
              approvers.insert(owner);
            }
          }
        }

        if approvers.is_empty() {
          approvers.insert(input.requestor);
        }
      }
    }

    if approvers.is_empty() {
      return Err(CoreError::InvalidInput(InvalidInput::Other(
        "request has no eligible approvers".to_string(),
      )));
    }

    let mut approvers_vec: Vec<Uuid> = approvers.into_iter().collect();
    approvers_vec.sort();

    let max_quorum = approvers_vec.len().max(1);
    let quorum = input.quorum.unwrap_or(1).min(max_quorum);

    Ok((approvers_vec, quorum))
  }

  pub async fn get_eligible_approvers(&self, id: Uuid) -> Result<Vec<Uuid>> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    let request = match node.metadata {
      NodeMeta::Request(request) => request,
      _ => return Err(CoreError::NotFound),
    };

    self.eligible_approvers_for_request(&request).await
  }

  async fn eligible_approvers_for_request(&self, request: &Request) -> Result<Vec<Uuid>> {
    let input = RequestInput {
      resource_id: request.resource_id,
      resource_type: request.resource_type.clone(),
      actions: request.actions.clone(),
      scope: request.scope.clone(),
      requestor: request.requestor,
      owners: request.owners.clone(),
      ownership: Some(request.ownership.clone()),
      quorum: Some(request.quorum),
      create_if_missing: Some(request.create_if_missing),
      relationship_requests: request.relationship_requests.clone(),
      expires_at: request.expires_at,
    };

    let (approvers, _) = self
      .resolve_approvers(&input, request.ownership.clone())
      .await?;
    Ok(approvers)
  }

  async fn owners_of(&self, target_id: Uuid) -> Result<Vec<Uuid>> {
    let query = Query::edges(
      EdgeQuery::incoming(EdgeType::Owns.as_str())
        .from(NodeQuery::new(NodeType::Identity.as_str()))
        .to(NodeQuery::any().filter(field("id").eq(target_id.to_string()))),
    );

    let elements = self.exec.query(query).await?;
    let mut owners = Vec::new();

    for el in elements {
      if let Element::Edge(edge) = el {
        owners.push(edge.from_id);
      }
    }

    Ok(owners)
  }

  async fn resolve_owners_for_apply(
    &self,
    request: &Request,
    resource_id: Uuid,
  ) -> Result<Vec<Uuid>> {
    match request.ownership {
      RequestOwnership::Requestor => Ok(vec![request.requestor]),
      RequestOwnership::Explicit => Ok(request.owners.clone().unwrap_or_default()),
      RequestOwnership::Identity => {
        let owners = self.owners_of(resource_id).await?;
        if owners.is_empty() {
          Ok(vec![request.requestor])
        } else {
          Ok(owners)
        }
      }
    }
  }

  async fn ensure_identity_exists(&self, id: Uuid) -> Result<()> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match node.metadata {
      NodeMeta::Identity(_) => Ok(()),
      _ => Err(CoreError::InvalidInput(InvalidInput::Other(
        "expected identity node".to_string(),
      ))),
    }
  }

  async fn ensure_resource_exists(&self, id: Uuid) -> Result<()> {
    let node = self
      .exec
      .get_node_by_id(id)
      .await?
      .ok_or(CoreError::NotFound)?;

    match node.metadata {
      NodeMeta::Resource(_) => Ok(()),
      _ => Err(CoreError::InvalidInput(InvalidInput::Other(
        "expected resource node".to_string(),
      ))),
    }
  }

  fn edge_props_for_type(edge_type: EdgeType, now: DateTime<Utc>) -> Result<EdgeProps> {
    let props = match edge_type {
      EdgeType::MemberOf => EdgeProps::MemberOf {
        since: Some(now),
        until: None,
      },
      EdgeType::RevokedBy => EdgeProps::RevokedBy {
        since: Some(now),
        until: None,
      },
      EdgeType::HasKey => EdgeProps::HasKey {
        since: Some(now),
        until: None,
      },
      EdgeType::Owns => EdgeProps::Owns {
        since: Some(now),
        until: None,
      },
      EdgeType::RequestedFor
      | EdgeType::ApprovedBy
      | EdgeType::DeniedBy
      | EdgeType::AppliesTo
      | EdgeType::HasApproval
      | EdgeType::HasDenial => {
        return Err(CoreError::InvalidInput(InvalidInput::Other(
          "relationship requests do not support request lifecycle edge types".to_string(),
        )));
      }
    };

    Ok(props)
  }

  pub async fn evaluate_request_policies(
    &self,
    requested_for: Uuid,
    actions: &[String],
  ) -> Result<PolicyDecision> {
    let policies = self.collect_policies_for_identity(requested_for).await?;
    Ok(Self::evaluate_policies(&policies, actions))
  }

  async fn collect_policies_for_identity(&self, start: Uuid) -> Result<Vec<PolicyMeta>> {
    let mut policies = Vec::new();
    let mut visited = BTreeSet::new();
    let mut queue = vec![start];

    while let Some(current) = queue.pop() {
      if !visited.insert(current) {
        continue;
      }

      let query = Query::edges(
        EdgeQuery::outgoing(EdgeType::MemberOf.as_str())
          .from(NodeQuery::any().filter(field("id").eq(current.to_string()))),
      );

      let elements = self.exec.query(query).await?;
      for el in elements {
        if let Element::Edge(edge) = el {
          let next_id = edge.to_id;
          if !visited.contains(&next_id) {
            queue.push(next_id);
          }
          if let Some(node) = self.exec.get_node_by_id(next_id).await?
            && let NodeMeta::Policy(policy) = node.metadata
          {
            policies.push(policy);
          }
        }
      }
    }

    Ok(policies)
  }

  fn evaluate_policies(policies: &[PolicyMeta], actions: &[String]) -> PolicyDecision {
    let mut saw_allow = false;

    for policy in policies {
      for rule in &policy.rules {
        if Self::rule_matches(rule, actions) {
          match rule.effect {
            PolicyEffect::Deny => return PolicyDecision::Denied,
            PolicyEffect::Allow => saw_allow = true,
          }
        }
      }
    }

    if saw_allow {
      PolicyDecision::Allowed
    } else {
      PolicyDecision::NotApplicable
    }
  }

  fn rule_matches(rule: &crate::model::policy::PolicyRule, actions: &[String]) -> bool {
    if rule.actions.is_empty() {
      return false;
    }

    for action in actions {
      let action = action.to_lowercase();
      for rule_action in &rule.actions {
        match rule_action {
          PolicyAction::All => return true,
          PolicyAction::Read if action == "read" => return true,
          PolicyAction::Write if action == "write" => return true,
          PolicyAction::Custom(custom) if action == custom.to_lowercase() => return true,
          _ => {}
        }
      }
    }

    false
  }
}
