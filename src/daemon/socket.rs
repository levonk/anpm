//! Local socket IPC for the apmw daemon.
//!
//! Uses Unix domain sockets on Linux/macOS and named pipes on Windows. The
//! protocol is a simple JSON line protocol: one JSON request per line, one
//! JSON response per line.
//!
//! Socket path: `${XDG_RUNTIME_DIR:-/tmp}/apmw.sock`

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use crate::error::{ApmwError, Result};

use super::jobs::{JobId, JobInfo, JobStatus};

/// Default socket path: `${XDG_RUNTIME_DIR:-/tmp}/apmw.sock`.
pub fn default_socket_path() -> PathBuf {
  match std::env::var("XDG_RUNTIME_DIR") {
    Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join("apmw.sock"),
    _ => PathBuf::from("/tmp").join("apmw.sock"),
  }
}

/// A request sent from the CLI to the daemon over the socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
  /// Ping the daemon (health check).
  Ping,
  /// Request the daemon status.
  Status,
  /// Shut the daemon down.
  Shutdown,
  /// Reload configuration (SIGHUP equivalent over IPC).
  ReloadConfig,
  /// List all background jobs.
  ListJobs,
  /// Cancel a specific job.
  CancelJob { id: JobId },
  /// Get the status of a specific job.
  GetJob { id: JobId },
  /// Create a new job.
  CreateJob { kind: String, description: String },
}

/// A response sent from the daemon to the CLI over the socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
  /// Pong reply to a ping.
  Pong,
  /// Daemon status.
  Status { running: bool, pid: Option<u32> },
  /// Acknowledged (e.g. shutdown/reload accepted).
  Ok,
  /// List of jobs.
  Jobs { jobs: Vec<JobInfo> },
  /// A single job's info.
  Job { info: Option<JobInfo> },
  /// A job id was created.
  JobCreated { id: JobId },
  /// A job's status.
  JobStatus { id: JobId, status: JobStatus },
  /// An error occurred.
  Error { message: String },
}

impl Response {
  /// Convenience constructor for an error response.
  pub fn error(msg: impl Into<String>) -> Self {
    Response::Error {
      message: msg.into(),
    }
  }
}

/// A trait for async request handlers.
pub trait RequestHandler: Send + Sync {
  /// Handle a request and return a response.
  fn handle(
    &self,
    req: Request,
  ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>>;
}

/// A local socket server that accepts connections and handles JSON-line
/// requests.
pub struct SocketServer {
  path: PathBuf,
}

impl SocketServer {
  /// Create a new server bound to the given socket path.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }

  /// Return the socket path.
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// Bind the listener and run the accept loop, calling `handler` for each
  /// request. This runs until the listener is closed or a fatal error occurs.
  pub async fn run(&self, handler: impl RequestHandler + 'static) -> Result<()> {
    let handler: std::sync::Arc<dyn RequestHandler> = std::sync::Arc::new(handler);
    self.run_inner(handler).await
  }

  #[cfg(unix)]
  async fn run_inner(&self, handler: std::sync::Arc<dyn RequestHandler>) -> Result<()> {
    // Remove stale socket file if present.
    if self.path.exists() {
      let _ = std::fs::remove_file(&self.path);
    }

    let listener = tokio::net::UnixListener::bind(&self.path)
      .map_err(|e| ApmwError::Daemon(format!("bind socket {}: {e}", self.path.display())))?;

    // Restrict permissions so the socket is not world-writable.
    {
      use std::os::unix::fs::PermissionsExt;
      let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
    }

    info!(path = %self.path.display(), "daemon listening on socket");

    loop {
      match listener.accept().await {
        Ok((stream, _peer)) => {
          let handler = handler.clone();
          tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, handler).await {
              error!(error = %e, "connection error");
            }
          });
        }
        Err(e) => {
          error!(error = %e, "accept error");
          return Err(ApmwError::Daemon(format!("accept: {e}")));
        }
      }
    }
  }

  #[cfg(not(unix))]
  async fn run_inner(&self, handler: std::sync::Arc<dyn RequestHandler>) -> Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;
    let path_str = self.path.to_string_lossy().to_string();
    info!(path = %path_str, "daemon listening on named pipe");
    loop {
      let server = ServerOptions::new()
        .first_pipe_instance(false)
        .create(&path_str)
        .map_err(|e| ApmwError::Daemon(format!("create pipe: {e}")))?;
      server
        .connect()
        .await
        .map_err(|e| ApmwError::Daemon(format!("pipe connect: {e}")))?;
      let handler = handler.clone();
      tokio::spawn(async move {
        if let Err(e) = handle_connection(server, handler).await {
          error!(error = %e, "connection error");
        }
      });
    }
  }
}

#[cfg(unix)]
type ServerStream = tokio::net::UnixStream;

#[cfg(not(unix))]
type ServerStream = tokio::net::windows::named_pipe::NamedPipeServer;

async fn handle_connection(
  stream: ServerStream,
  handler: std::sync::Arc<dyn RequestHandler>,
) -> std::io::Result<()> {
  let (reader, mut writer) = stream.into_split();
  let mut lines = BufReader::new(reader).lines();
  while let Ok(Some(line)) = lines.next_line().await {
    if line.is_empty() {
      continue;
    }
    debug!(line = %line, "request");
    let response = match serde_json::from_str::<Request>(&line) {
      Ok(req) => handler.handle(req).await,
      Err(e) => Response::error(format!("invalid request: {e}")),
    };
    let mut resp_json = serde_json::to_vec(&response)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    resp_json.push(b'\n');
    writer.write_all(&resp_json).await?;
  }
  Ok(())
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// A client that connects to the daemon socket and sends a single request.
pub struct SocketClient {
  path: PathBuf,
}

impl SocketClient {
  /// Create a new client targeting the given socket path.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }

  /// Send a request and read a single response line.
  pub async fn send(&self, request: &Request) -> Result<Response> {
    let req_json = serde_json::to_string(request)?;
    let response = self.send_raw(&req_json).await?;
    let resp: Response = serde_json::from_str(&response)?;
    Ok(resp)
  }

  /// Send a raw JSON line and read a single response line.
  pub async fn send_raw(&self, json: &str) -> Result<String> {
    self.connect_and_exchange(json).await
  }

  #[cfg(unix)]
  async fn connect_and_exchange(&self, json: &str) -> Result<String> {
    let mut stream = tokio::net::UnixStream::connect(&self.path)
      .await
      .map_err(|e| ApmwError::Daemon(format!("connect {}: {e}", self.path.display())))?;
    stream
      .write_all(json.as_bytes())
      .await
      .map_err(|e| ApmwError::Daemon(format!("write: {e}")))?;
    stream
      .write_all(b"\n")
      .await
      .map_err(|e| ApmwError::Daemon(format!("write nl: {e}")))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
      .read_line(&mut line)
      .await
      .map_err(|e| ApmwError::Daemon(format!("read: {e}")))?;
    Ok(line.trim_end().to_string())
  }

  #[cfg(not(unix))]
  async fn connect_and_exchange(&self, json: &str) -> Result<String> {
    use tokio::net::windows::named_pipe::ClientOptions;
    let mut stream = ClientOptions::new()
      .open(self.path.to_string_lossy().as_ref())
      .await
      .map_err(|e| ApmwError::Daemon(format!("open pipe: {e}")))?;
    stream
      .write_all(json.as_bytes())
      .await
      .map_err(|e| ApmwError::Daemon(format!("write: {e}")))?;
    stream
      .write_all(b"\n")
      .await
      .map_err(|e| ApmwError::Daemon(format!("write nl: {e}")))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
      .read_line(&mut line)
      .await
      .map_err(|e| ApmwError::Daemon(format!("read: {e}")))?;
    Ok(line.trim_end().to_string())
  }

  /// Check whether the daemon is reachable by sending a ping.
  pub async fn ping(&self) -> Result<bool> {
    match self.send(&Request::Ping).await {
      Ok(Response::Pong) => Ok(true),
      Ok(_) => Ok(false),
      Err(_) => Ok(false),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  /// A simple test handler that returns canned responses.
  struct TestHandler;

  impl RequestHandler for TestHandler {
    fn handle(
      &self,
      req: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
      Box::pin(async move {
        match req {
          Request::Ping => Response::Pong,
          Request::Status => Response::Status {
            running: true,
            pid: Some(std::process::id()),
          },
          Request::Shutdown => Response::Ok,
          Request::ReloadConfig => Response::Ok,
          Request::ListJobs => Response::Jobs { jobs: vec![] },
          Request::CancelJob { id } => Response::JobStatus {
            id,
            status: JobStatus::Cancelled,
          },
          Request::GetJob { .. } => Response::Job { info: None },
          Request::CreateJob {
            kind,
            description: _,
          } => Response::JobCreated {
            id: JobId::new(kind),
          },
        }
      })
    }
  }

  async fn start_server(path: PathBuf) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
      let server = SocketServer::new(path);
      let _ = server.run(TestHandler).await;
    });
    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    handle
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_ping() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.sock");
    let _handle = start_server(path.clone()).await;
    let client = SocketClient::new(&path);
    let resp = client.send(&Request::Ping).await.unwrap();
    assert!(matches!(resp, Response::Pong));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_status() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("status.sock");
    let _handle = start_server(path.clone()).await;
    let client = SocketClient::new(&path);
    let resp = client.send(&Request::Status).await.unwrap();
    match resp {
      Response::Status { running, pid } => {
        assert!(running);
        assert!(pid.is_some());
      }
      _ => panic!("expected Status response"),
    }
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_cancel_job() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cancel.sock");
    let _handle = start_server(path.clone()).await;
    let client = SocketClient::new(&path);
    let resp = client
      .send(&Request::CancelJob {
        id: JobId::new("abc"),
      })
      .await
      .unwrap();
    match resp {
      Response::JobStatus { id, status } => {
        assert_eq!(id.as_str(), "abc");
        assert_eq!(status, JobStatus::Cancelled);
      }
      _ => panic!("expected JobStatus response"),
    }
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_list_jobs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("list.sock");
    let _handle = start_server(path.clone()).await;
    let client = SocketClient::new(&path);
    let resp = client.send(&Request::ListJobs).await.unwrap();
    match resp {
      Response::Jobs { jobs } => assert!(jobs.is_empty()),
      _ => panic!("expected Jobs response"),
    }
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_invalid_request() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("invalid.sock");
    let _handle = start_server(path.clone()).await;
    let client = SocketClient::new(&path);
    let resp = client.send_raw("{not valid json}").await.unwrap();
    assert!(resp.contains("invalid request"));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_socket_ping_unreachable() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nope.sock");
    let client = SocketClient::new(&path);
    let reachable = client.ping().await.unwrap();
    assert!(!reachable);
  }

  #[test]
  fn test_default_socket_path() {
    let path = default_socket_path();
    assert!(path.to_string_lossy().ends_with("apmw.sock"));
  }

  #[test]
  fn test_request_response_serde() {
    let req = Request::Ping;
    let json = serde_json::to_string(&req).unwrap();
    let back: Request = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, Request::Ping));

    let resp = Response::Pong;
    let json = serde_json::to_string(&resp).unwrap();
    let back: Response = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, Response::Pong));
  }
}
