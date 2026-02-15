use tonic::Status;
use url::Url;
use uuid::Uuid;

use mises_graph::{EdgeQuery, Element, Filter, NodeQuery, Query, field};

use mises_core::{
  CoreError,
  model::{edge::EdgeType, identity::IdentityType, node::NodeType},
  service::identity::IdentityService,
  traits::Repository,
};

pub fn matches_redirect_pattern(redirect_uri: &str, pattern: &str) -> bool {
  if pattern == redirect_uri {
    return true;
  }

  let Ok(redirect_url) = Url::parse(redirect_uri) else {
    return false;
  };
  let (pattern_url, scheme_present, path_present) = if pattern.contains("://") {
    let Ok(parsed) = Url::parse(pattern) else {
      return false;
    };
    (parsed, true, true)
  } else {
    let Ok(parsed) = Url::parse(&format!("https://{}", pattern)) else {
      return false;
    };
    (parsed, false, pattern.contains('/'))
  };

  if scheme_present && redirect_url.scheme() != pattern_url.scheme() {
    return false;
  }

  if let (Some(redirect_host), Some(pattern_host)) =
    (redirect_url.host_str(), pattern_url.host_str())
  {
    if let Some(domain_suffix) = pattern_host.strip_prefix("*.") {
      if domain_suffix.is_empty() {
        return false;
      }
      if !redirect_host.ends_with(domain_suffix) {
        return false;
      }
      if redirect_host != domain_suffix && !redirect_host.ends_with(&format!(".{}", domain_suffix))
      {
        return false;
      }
    } else if redirect_host != pattern_host {
      return false;
    }
  } else {
    return false;
  }

  if (scheme_present || pattern_url.port().is_some()) && redirect_url.port() != pattern_url.port() {
    return false;
  }

  let redirect_path = redirect_url.path();
  let pattern_path = pattern_url.path();

  if scheme_present || path_present {
    if let Some(prefix) = pattern_path.strip_suffix("/*") {
      if !redirect_path.starts_with(prefix) {
        return false;
      }
    } else if redirect_path != pattern_path {
      return false;
    }
  }

  if pattern_url.query().is_some() {
    if redirect_url.query() != pattern_url.query() {
      return false;
    }
  } else if scheme_present && redirect_url.query() != pattern_url.query() {
    return false;
  }

  true
}

pub async fn ensure_application_identity<R>(repo: &R, client_uuid: Uuid) -> Result<R::Node, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let identity_service = IdentityService::new(repo.clone());
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

pub async fn ensure_service_identity<R>(repo: &R, service_id: &str) -> Result<R::Node, Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let query = Query::nodes(
    NodeQuery::new(NodeType::Identity.as_str()).filter(Filter::all([
      field("metadata.type")
        .eq(IdentityType::Service.as_str())
        .into(),
      field("metadata.name").eq(service_id.to_string()).into(),
    ])),
  );

  let elements = repo
    .query(query)
    .await
    .map_err(|e| Status::internal(format!("identity lookup error: {}", e)))?;

  let mut matches = elements.into_iter().filter_map(|el| match el {
    Element::Node(node) => Some(node),
    _ => None,
  });

  let Some(node) = matches.next() else {
    return Err(Status::invalid_argument(format!(
      "invalid_request: service_id not found: {}",
      service_id
    )));
  };

  if matches.next().is_some() {
    return Err(Status::invalid_argument(
      "invalid_request: service_id is not unique",
    ));
  }

  Ok(node)
}

pub async fn ensure_service_owns_application<R>(
  repo: &R,
  service_id: Uuid,
  application_id: Uuid,
) -> Result<(), Status>
where
  R: Repository + Clone + Send + Sync + 'static,
{
  let query = Query::edges(
    EdgeQuery::outgoing(EdgeType::Owns.as_str())
      .from(NodeQuery::any().filter(field("id").eq(service_id.to_string())))
      .to(NodeQuery::any().filter(field("id").eq(application_id.to_string()))),
  );

  let elements = repo
    .query(query)
    .await
    .map_err(|e| Status::internal(format!("ownership check failed: {}", e)))?;

  for el in elements {
    if let Element::Edge(edge) = el
      && edge.r#type == EdgeType::Owns.as_str()
      && edge.from_id == service_id
      && edge.to_id == application_id
    {
      return Ok(());
    }
  }

  Err(Status::invalid_argument(
    "invalid_request: service does not own client application",
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_exact_match() {
    assert!(matches_redirect_pattern(
      "https://example.com/callback",
      "https://example.com/callback"
    ));
    assert!(!matches_redirect_pattern(
      "https://example.com/other",
      "https://example.com/callback"
    ));
  }

  #[test]
  fn test_subdomain_wildcard() {
    assert!(matches_redirect_pattern(
      "https://app.test.com/callback",
      "https://*.test.com/callback"
    ));
    assert!(matches_redirect_pattern(
      "https://api.test.com/callback",
      "https://*.test.com/callback"
    ));
    assert!(matches_redirect_pattern(
      "https://test.com/callback",
      "https://*.test.com/callback"
    ));
    assert!(!matches_redirect_pattern(
      "https://test.com/callback",
      "https://*.example.com/callback"
    ));
    assert!(!matches_redirect_pattern(
      "https://malicious-test.com/callback",
      "https://*.test.com/callback"
    ));
  }

  #[test]
  fn test_path_wildcard() {
    assert!(matches_redirect_pattern(
      "https://www.test.com/path/callback",
      "https://www.test.com/path/*"
    ));
    assert!(matches_redirect_pattern(
      "https://www.test.com/path/admin/callback",
      "https://www.test.com/path/*"
    ));
    assert!(matches_redirect_pattern(
      "https://www.test.com/path/",
      "https://www.test.com/path/*"
    ));
    assert!(!matches_redirect_pattern(
      "https://www.test.com/other/callback",
      "https://www.test.com/path/*"
    ));
  }

  #[test]
  fn test_scheme_less_patterns() {
    assert!(matches_redirect_pattern(
      "https://www.test.com/path/callback",
      "www.test.com/path/*"
    ));
    assert!(matches_redirect_pattern(
      "http://api.test.com/other",
      "*.test.com"
    ));
    assert!(!matches_redirect_pattern(
      "https://test.com.evil.com/path",
      "*.test.com"
    ));
  }

  #[test]
  fn test_scheme_less_port() {
    assert!(matches_redirect_pattern(
      "https://example.com:8080/callback",
      "example.com:8080/*"
    ));
    assert!(!matches_redirect_pattern(
      "https://example.com:9090/callback",
      "example.com:8080/*"
    ));
  }

  #[test]
  fn test_scheme_mismatch() {
    assert!(!matches_redirect_pattern(
      "http://example.com/callback",
      "https://example.com/callback"
    ));
  }

  #[test]
  fn test_port_matching() {
    assert!(matches_redirect_pattern(
      "https://example.com:8080/callback",
      "https://example.com:8080/callback"
    ));
    assert!(!matches_redirect_pattern(
      "https://example.com:8080/callback",
      "https://example.com:9090/callback"
    ));
  }

  #[test]
  fn test_combined_wildcards() {
    assert!(matches_redirect_pattern(
      "https://app.test.com/auth/callback",
      "https://*.test.com/auth/*"
    ));
    assert!(!matches_redirect_pattern(
      "https://app.example.com/auth/callback",
      "https://*.test.com/auth/*"
    ));
  }
}
