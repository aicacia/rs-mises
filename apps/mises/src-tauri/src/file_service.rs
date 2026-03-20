use std::{convert::Infallible, sync::Arc};

use hyper::{Body, Client, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use mises_file_store::{FileMetadata, FileStore, FsFileStore};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::FileServiceConfig;

pub type AccessChecker = Arc<dyn Fn(Uuid, Uuid, FileAccessOperation) -> bool + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccessOperation {
    Read,
    Write,
}

pub struct FileServiceState {
    store: FsFileStore,
    access_checker: Option<AccessChecker>,
}

impl FileServiceState {
    fn allowed(&self, identity: Uuid, resource: Uuid, op: FileAccessOperation) -> bool {
        self.access_checker
            .as_ref()
            .map(|checker| checker(identity, resource, op))
            .unwrap_or(true)
    }
}

fn pick_local_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("alloc port")
        .local_addr()
        .expect("local addr")
        .port()
}

pub async fn start_file_service(
    config: FileServiceConfig,
    access_checker: Option<AccessChecker>,
    cancellation_token: CancellationToken,
) {
    if !config.enabled {
        log::info!("file service is disabled");
        return;
    }

    let store = match FsFileStore::new(&config.root_dir) {
        Ok(store) => store,
        Err(e) => {
            log::error!("failed to initialize file store: {:?}", e);
            return;
        }
    };

    let state = Arc::new(FileServiceState { store, access_checker });

    let addr = format!("{}:{}", config.bind_host, config.bind_port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => {
            log::info!("file service listening on http://{} (root={:?})", addr, config.root_dir);
            listener
        }
        Err(e) => {
            log::error!("failed to bind file service at {}: {}", addr, e);
            return;
        }
    };

    let std_listener = listener
        .into_std()
        .expect("failed to convert tokio listener into std listener");

    let server = Server::from_tcp(std_listener)
        .unwrap()
        .serve(make_service_fn(move |_| {
            let state = state.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| handle_request(req, state.clone())))
            }
        }));

    let graceful = server.with_graceful_shutdown(async move {
        cancellation_token.cancelled().await;
        log::info!("file service cancellation received");
    });

    if let Err(e) = graceful.await {
        log::error!("file service terminated with error: {}", e);
    }
}

fn parse_identity(req: &Request<Body>) -> Option<Uuid> {
    req.headers().get("authorization").and_then(|value: &hyper::header::HeaderValue| {
        let value = value.to_str().ok()?;
        let token = value.strip_prefix("Bearer ")?;
        Uuid::parse_str(token).ok()
    })
}

async fn handle_request(
    req: Request<Body>,
    state: Arc<FileServiceState>,
) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if segments.len() < 3 || segments[0] != "resources" || segments[2] != "files" {
        return Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap());
    }

    let resource_id = match Uuid::parse_str(segments[1]) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(Response::builder().status(StatusCode::BAD_REQUEST).body(Body::from("invalid resource id")).unwrap());
        }
    };

    let identity_id = match parse_identity(&req) {
        Some(uuid) => uuid,
        None => {
            return Ok(Response::builder().status(StatusCode::UNAUTHORIZED).body(Body::from("missing or invalid authorization token")).unwrap());
        }
    };

    let op = match *req.method() {
        Method::GET => FileAccessOperation::Read,
        Method::PUT | Method::DELETE => FileAccessOperation::Write,
        _ => {
            return Ok(Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).body(Body::empty()).unwrap());
        }
    };

    if !state.allowed(identity_id, resource_id, op) {
        return Ok(Response::builder().status(StatusCode::FORBIDDEN).body(Body::from("forbidden")).unwrap());
    }

    let file_path = if segments.len() > 3 { Some(segments[3..].join("/")) } else { None };

    match (req.method(), file_path.as_deref()) {
        (&Method::GET, Some(fp)) => handle_get_file(&state, &resource_id, fp).await,
        (&Method::GET, None) => handle_list_files(&state, &resource_id, req.uri().query()).await,
        (&Method::PUT, Some(fp)) => handle_put_file(&state, &resource_id, fp, req).await,
        (&Method::DELETE, Some(fp)) => handle_delete_file(&state, &resource_id, fp).await,
        _ => Ok(Response::builder().status(StatusCode::BAD_REQUEST).body(Body::from("invalid path")).unwrap()),
    }
}

async fn handle_list_files(
    state: &FileServiceState,
    resource_id: &Uuid,
    query: Option<&str>,
) -> Result<Response<Body>, Infallible> {
    let prefix = query.and_then(|q| {
        q.split('&').find_map(|part| {
            let mut split = part.split('=');
            let key = split.next()?;
            let value = split.next()?;
            if key == "prefix" { Some(value) } else { None }
        })
    });

    match state.store.list(&resource_id.to_string(), prefix) {
        Ok(files) => {
            let body = serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string());
            Ok(Response::builder().status(StatusCode::OK).header("content-type", "application/json").body(Body::from(body)).unwrap())
        }
        Err(_) => Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("list error")).unwrap()),
    }
}

async fn handle_get_file(
    state: &FileServiceState,
    resource_id: &Uuid,
    path: &str,
) -> Result<Response<Body>, Infallible> {
    match state.store.get(&resource_id.to_string(), path) {
        Ok(Some(file)) => {
            let mut response = Response::builder().status(StatusCode::OK).body(Body::from(file.data)).unwrap();
            if let Some(metadata) = file.metadata {
                if let Some(content_type) = metadata.content_type {
                    response.headers_mut().insert("content-type", content_type.parse().unwrap());
                }
            }
            Ok(response)
        }
        Ok(None) => Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()),
        Err(_) => Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("get error")).unwrap()),
    }
}

async fn handle_put_file(
    state: &FileServiceState,
    resource_id: &Uuid,
    path: &str,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|value: &hyper::header::HeaderValue| value.to_str().ok())
        .map(str::to_string);

    let data = hyper::body::to_bytes(req.into_body()).await.unwrap_or_default();

    let metadata = FileMetadata {
        content_type,
        size: data.len() as u64,
        created_at: None,
        updated_at: None,
        tags: None,
    };

    match state.store.put(&resource_id.to_string(), path, data.as_ref(), &metadata) {
        Ok(_) => Ok(Response::builder().status(StatusCode::CREATED).body(Body::empty()).unwrap()),
        Err(_) => Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("put error")).unwrap()),
    }
}

async fn handle_delete_file(
    state: &FileServiceState,
    resource_id: &Uuid,
    path: &str,
) -> Result<Response<Body>, Infallible> {
    match state.store.delete(&resource_id.to_string(), path) {
        Ok(_) => Ok(Response::builder().status(StatusCode::NO_CONTENT).body(Body::empty()).unwrap()),
        Err(_) => Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::from("delete error")).unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn disabled_file_service_immediate() {
        let config = FileServiceConfig {
            enabled: false,
            root_dir: PathBuf::from("./tmp-file-service-disabled"),
            bind_host: "127.0.0.1".to_string(),
            bind_port: 0,
        };

        let token = CancellationToken::new();
        start_file_service(config, None, token).await;
    }

    #[tokio::test]
    async fn enabled_file_service_can_launch_and_stop() {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        let temp_root = std::env::temp_dir().join(format!("mises-file-service-test-{}", now));
        let config = FileServiceConfig {
            enabled: true,
            root_dir: temp_root,
            bind_host: "127.0.0.1".to_string(),
            bind_port: 0,
        };

        let token = CancellationToken::new();
        let child_token = token.clone();

        let handle = tokio::spawn(async move {
            start_file_service(config, None, child_token).await;
        });

        sleep(Duration::from_millis(100)).await;
        token.cancel();
        let _ = handle.await;
    }

  #[tokio::test]
  async fn e2e_file_crud_with_access_control() {
    let root_dir = std::env::temp_dir().join(format!("mises-file-service-e2e-{}", pick_local_port()));
    let port = pick_local_port();
    let config = FileServiceConfig {
      enabled: true,
      root_dir: root_dir.clone(),
      bind_host: "127.0.0.1".to_string(),
      bind_port: port,
    };

    let token = CancellationToken::new();
    let child_token = token.clone();

    let checker: AccessChecker = Arc::new(move |_identity, _resource, _op| true);

    let handle = tokio::spawn(async move {
      start_file_service(config, Some(checker.clone()), child_token).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let identity = Uuid::new_v4();
    let resource_id = Uuid::new_v4();
    let client = hyper::Client::new();

    let file_url = format!("http://127.0.0.1:{}/resources/{}/files/test.txt", port, resource_id);
    let req = Request::builder()
      .method(Method::PUT)
      .uri(&file_url)
      .header("authorization", format!("Bearer {}", identity))
      .body(Body::from("hello"))
      .unwrap();

    let resp: Response<Body> = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let get_url = format!("http://127.0.0.1:{}/resources/{}/files/test.txt", port, resource_id);
    let req = Request::builder()
      .method(Method::GET)
      .uri(&get_url)
      .header("authorization", format!("Bearer {}", identity))
      .body(Body::empty())
      .unwrap();

    let resp: Response<Body> = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body_bytes, "hello");

    let delete_url = format!("http://127.0.0.1:{}/resources/{}/files/test.txt", port, resource_id);
    let req = Request::builder()
      .method(Method::DELETE)
      .uri(&delete_url)
      .header("authorization", format!("Bearer {}", identity))
      .body(Body::empty())
      .unwrap();

    let resp: Response<Body> = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    token.cancel();
    let _ = handle.await;
  }
}
