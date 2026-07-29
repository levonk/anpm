//! Daemon manager for apmw.
//!
//! Implements the daemon skeleton per ADR-20260607001 §13: a long-running
//! tokio process that listens on a local socket, manages background jobs, and
//! handles SIGINT (exit 130) and SIGHUP (config reload).
//!
//! The daemon can run in two modes:
//! - **Process mode**: spawned as a separate background process via
//!   [`DaemonManager::auto_spawn`].
//! - **In-process mode**: runs in a tokio task within the current process
//!   (used for testing and `--no-daemon` synchronous fallback).

pub mod jobs;
pub mod socket;

pub use jobs::{JobId, JobInfo, JobKind, JobManager, JobStatus};
pub use socket::{default_socket_path, Request, Response, SocketClient, SocketServer};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::error::{ApmwError, Result};

/// Exit code used when the daemon is terminated by SIGINT (128 + 2).
pub const SIGINT_EXIT_CODE: i32 = 130;

/// State shared between the socket handler and the daemon runtime.
pub struct DaemonState {
  /// Background job manager.
  pub jobs: JobManager,
  /// Whether a shutdown has been requested.
  pub shutdown_requested: Mutex<bool>,
}

impl DaemonState {
  /// Create new shared daemon state.
  pub fn new() -> Self {
    Self {
      jobs: JobManager::new(),
      shutdown_requested: Mutex::new(false),
    }
  }

  /// Handle a single IPC request and produce a response.
  pub async fn handle_request(&self, req: Request) -> Response {
    match req {
      Request::Ping => Response::Pong,
      Request::Status => Response::Status {
        running: true,
        pid: Some(std::process::id()),
      },
      Request::Shutdown => {
        *self.shutdown_requested.lock().await = true;
        info!("shutdown requested via IPC");
        Response::Ok
      }
      Request::ReloadConfig => {
        info!("config reload requested via IPC");
        // Actual reload is a no-op until config stories wire in live reload.
        Response::Ok
      }
      Request::ListJobs => {
        let jobs = self.jobs.list_jobs().await;
        Response::Jobs { jobs }
      }
      Request::CancelJob { id } => match self.jobs.cancel_job(&id).await {
        Ok(()) => Response::JobStatus {
          id,
          status: JobStatus::Cancelled,
        },
        Err(e) => Response::error(e),
      },
      Request::GetJob { id } => {
        let info = self.jobs.get_job_info(&id).await;
        Response::Job { info }
      }
      Request::CreateJob { kind, description } => {
        let id = self.jobs.create_job(&kind, &description).await;
        Response::JobCreated { id }
      }
    }
  }

  /// Returns `true` if a shutdown has been requested.
  pub async fn shutdown_requested(&self) -> bool {
    *self.shutdown_requested.lock().await
  }
}

impl Default for DaemonState {
  fn default() -> Self {
    Self::new()
  }
}

/// A `RequestHandler` backed by shared `DaemonState`.
struct StateHandler {
  state: Arc<DaemonState>,
}

impl socket::RequestHandler for StateHandler {
  fn handle(
    &self,
    req: Request,
  ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
    let state = self.state.clone();
    Box::pin(async move { state.handle_request(req).await })
  }
}

/// Manages the apmw daemon lifecycle.
pub struct DaemonManager {
  socket_path: PathBuf,
}

impl DaemonManager {
  /// Create a new `DaemonManager` using the default socket path.
  pub fn new() -> Self {
    Self {
      socket_path: default_socket_path(),
    }
  }

  /// Create a new `DaemonManager` with a custom socket path (useful for
  /// tests).
  pub fn with_socket_path(path: impl Into<PathBuf>) -> Self {
    Self {
      socket_path: path.into(),
    }
  }

  /// Return the socket path in use.
  pub fn socket_path(&self) -> &Path {
    &self.socket_path
  }

  /// Start the daemon in-process: bind the socket and run the accept loop
  /// until a shutdown is requested or a fatal error occurs. This is the
  /// main entry point for the daemon process and for in-process (test) mode.
  pub async fn start(&self) -> Result<()> {
    let state = Arc::new(DaemonState::new());
    self.start_with_state(state).await
  }

  /// Start the daemon with a pre-created shared state (useful for tests that
  /// need to inspect job state after the daemon stops).
  pub async fn start_with_state(&self, state: Arc<DaemonState>) -> Result<()> {
    // Spawn the signal handler task.
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown.clone();
    let state_for_signals = state.clone();

    #[cfg(unix)]
    let signal_handle = {
      tokio::spawn(async move {
        if let Err(e) = install_signal_handlers(shutdown_clone, state_for_signals).await {
          error!(error = %e, "signal handler error");
        }
      })
    };

    // Spawn the socket server task.
    let server = SocketServer::new(self.socket_path.clone());
    let state_for_server = state.clone();
    let shutdown_for_server = shutdown.clone();

    let server_task = tokio::spawn(async move {
      let handler = StateHandler {
        state: state_for_server,
      };
      let result = server.run(handler).await;
      if let Err(e) = &result {
        error!(error = %e, "socket server error");
      }
      // Signal the main loop to exit if the server stops.
      shutdown_for_server.notify_waiters();
      result
    });

    // Wait for either a shutdown request or the server to stop.
    shutdown.notified().await;

    // Clean up the socket file.
    #[cfg(unix)]
    {
      if self.socket_path.exists() {
        let _ = std::fs::remove_file(&self.socket_path);
      }
    }

    // Abort the server task.
    server_task.abort();
    #[cfg(unix)]
    {
      signal_handle.abort();
    }

    info!("daemon stopped");
    Ok(())
  }

  /// Stop a running daemon by sending a shutdown request over the socket.
  pub async fn stop(&self) -> Result<()> {
    let client = SocketClient::new(&self.socket_path);
    match client.send(&Request::Shutdown).await {
      Ok(Response::Ok) => {
        info!("daemon stop request sent");
        Ok(())
      }
      Ok(other) => Err(ApmwError::Daemon(format!(
        "unexpected response to shutdown: {other:?}"
      ))),
      Err(e) => Err(e),
    }
  }

  /// Query the daemon status over the socket.
  pub async fn status(&self) -> Result<DaemonStatus> {
    let client = SocketClient::new(&self.socket_path);
    match client.send(&Request::Status).await {
      Ok(Response::Status { running, pid }) => Ok(DaemonStatus { running, pid }),
      Ok(_) => Ok(DaemonStatus {
        running: false,
        pid: None,
      }),
      Err(_) => Ok(DaemonStatus {
        running: false,
        pid: None,
      }),
    }
  }

  /// Check if the daemon is currently running by attempting a ping.
  pub async fn is_running(&self) -> bool {
    let client = SocketClient::new(&self.socket_path);
    client.ping().await.unwrap_or(false)
  }

  /// Auto-spawn the daemon if it is not already running.
  ///
  /// In production this spawns a new `apmw --daemon` process. For testing,
  /// use [`DaemonManager::auto_spawn_in_process`] which runs the daemon in a
  /// tokio task within the current process.
  pub async fn auto_spawn(&self) -> Result<()> {
    if self.is_running().await {
      return Ok(());
    }
    info!("daemon not running, spawning new process");
    let exe = std::env::current_exe()
      .map_err(|e| ApmwError::Daemon(format!("cannot find current exe: {e}")))?;
    // Best-effort spawn; detach the child.
    match std::process::Command::new(&exe)
      .arg("--daemon")
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .stdin(std::process::Stdio::null())
      .spawn()
    {
      Ok(child) => {
        info!(pid = child.id(), "spawned daemon process");
        // Wait briefly for the daemon to bind the socket.
        for _ in 0..50 {
          if self.is_running().await {
            return Ok(());
          }
          tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        Err(ApmwError::Daemon(
          "daemon process spawned but did not become reachable".to_string(),
        ))
      }
      Err(e) => Err(ApmwError::Daemon(format!("failed to spawn daemon: {e}"))),
    }
  }

  /// Auto-spawn the daemon in-process (as a tokio task) for testing.
  ///
  /// Returns a `JoinHandle` that can be awaited when the test is done, plus
  /// shared state for inspection.
  pub fn auto_spawn_in_process(&self) -> (tokio::task::JoinHandle<()>, Arc<DaemonState>) {
    let state = Arc::new(DaemonState::new());
    let state_for_task = state.clone();
    let path = self.socket_path.clone();
    let handle = tokio::spawn(async move {
      let manager = DaemonManager::with_socket_path(path);
      if let Err(e) = manager.start_with_state(state_for_task).await {
        error!(error = %e, "in-process daemon exited with error");
      }
    });
    (handle, state)
  }

  /// List jobs by querying the daemon over the socket.
  pub async fn list_jobs(&self) -> Result<Vec<JobInfo>> {
    let client = SocketClient::new(&self.socket_path);
    match client.send(&Request::ListJobs).await {
      Ok(Response::Jobs { jobs }) => Ok(jobs),
      Ok(other) => Err(ApmwError::Daemon(format!(
        "unexpected response to list-jobs: {other:?}"
      ))),
      Err(e) => Err(e),
    }
  }

  /// Cancel a job by id over the socket.
  pub async fn cancel_job(&self, id: &JobId) -> Result<JobStatus> {
    let client = SocketClient::new(&self.socket_path);
    match client.send(&Request::CancelJob { id: id.clone() }).await {
      Ok(Response::JobStatus { status, .. }) => Ok(status),
      Ok(Response::Error { message }) => Err(ApmwError::Daemon(message)),
      Ok(other) => Err(ApmwError::Daemon(format!(
        "unexpected response to cancel-job: {other:?}"
      ))),
      Err(e) => Err(e),
    }
  }
}

impl Default for DaemonManager {
  fn default() -> Self {
    Self::new()
  }
}

/// Snapshot of the daemon's status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
  /// Whether the daemon is running and reachable.
  pub running: bool,
  /// OS process id of the daemon, if known.
  pub pid: Option<u32>,
}

// ---------------------------------------------------------------------------
// Signal handling
// ---------------------------------------------------------------------------

/// Install SIGINT and SIGHUP handlers (Unix only).
///
/// - SIGINT: request shutdown and exit with code 130.
/// - SIGHUP: trigger a config reload (logged).
#[cfg(unix)]
async fn install_signal_handlers(
  shutdown: Arc<tokio::sync::Notify>,
  state: Arc<DaemonState>,
) -> Result<()> {
  use tokio::signal::unix::{signal, SignalKind};

  let mut sigint = signal(SignalKind::interrupt())
    .map_err(|e| ApmwError::Daemon(format!("install SIGINT handler: {e}")))?;
  let mut sighup = signal(SignalKind::hangup())
    .map_err(|e| ApmwError::Daemon(format!("install SIGHUP handler: {e}")))?;

  loop {
    tokio::select! {
      _ = sigint.recv() => {
        info!("received SIGINT, shutting down");
        *state.shutdown_requested.lock().await = true;
        shutdown.notify_waiters();
        // Exit with code 130 (128 + SIGINT=2).
        std::process::exit(SIGINT_EXIT_CODE);
      }
      _ = sighup.recv() => {
        info!("received SIGHUP, reloading config");
        // Actual config reload is a no-op until config stories wire in live
        // reload. We log the event for observability.
        warn!("config reload requested but not yet implemented");
      }
    }
  }
}

#[cfg(not(unix))]
/// On non-Unix platforms, install a Ctrl-C handler that requests shutdown.
async fn install_signal_handlers(
  shutdown: Arc<tokio::sync::Notify>,
  state: Arc<DaemonState>,
) -> Result<()> {
  let _ = state;
  tokio::signal::ctrl_c()
    .await
    .map_err(|e| ApmwError::Daemon(format!("ctrl-c handler: {e}")))?;
  info!("received Ctrl-C, shutting down");
  shutdown.notify_waiters();
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[tokio::test]
  async fn test_daemon_state_handle_ping() {
    let state = DaemonState::new();
    let resp = state.handle_request(Request::Ping).await;
    assert!(matches!(resp, Response::Pong));
  }

  #[tokio::test]
  async fn test_daemon_state_handle_status() {
    let state = DaemonState::new();
    let resp = state.handle_request(Request::Status).await;
    match resp {
      Response::Status { running, pid } => {
        assert!(running);
        assert!(pid.is_some());
      }
      _ => panic!("expected Status response"),
    }
  }

  #[tokio::test]
  async fn test_daemon_state_handle_create_and_list_jobs() {
    let state = DaemonState::new();
    let resp = state
      .handle_request(Request::CreateJob {
        kind: "clone".to_string(),
        description: "clone repo".to_string(),
      })
      .await;
    let id = match resp {
      Response::JobCreated { id } => id,
      _ => panic!("expected JobCreated response"),
    };
    let resp = state.handle_request(Request::ListJobs).await;
    match resp {
      Response::Jobs { jobs } => {
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, id);
      }
      _ => panic!("expected Jobs response"),
    }
  }

  #[tokio::test]
  async fn test_daemon_state_handle_cancel_job() {
    let state = DaemonState::new();
    let id = state.jobs.create_job("scan", "scan project").await;
    let resp = state
      .handle_request(Request::CancelJob { id: id.clone() })
      .await;
    match resp {
      Response::JobStatus { id: rid, status } => {
        assert_eq!(rid, id);
        assert_eq!(status, JobStatus::Cancelled);
      }
      _ => panic!("expected JobStatus response"),
    }
  }

  #[tokio::test]
  async fn test_daemon_state_handle_cancel_unknown_job() {
    let state = DaemonState::new();
    let resp = state
      .handle_request(Request::CancelJob {
        id: JobId::new("nope"),
      })
      .await;
    assert!(matches!(resp, Response::Error { .. }));
  }

  #[tokio::test]
  async fn test_daemon_state_handle_shutdown() {
    let state = Arc::new(DaemonState::new());
    let resp = state.handle_request(Request::Shutdown).await;
    assert!(matches!(resp, Response::Ok));
    assert!(state.shutdown_requested().await);
  }

  #[tokio::test]
  async fn test_daemon_state_handle_reload_config() {
    let state = DaemonState::new();
    let resp = state.handle_request(Request::ReloadConfig).await;
    assert!(matches!(resp, Response::Ok));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_in_process_daemon_ping() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("daemon.sock");
    let manager = DaemonManager::with_socket_path(&path);
    let (handle, _state) = manager.auto_spawn_in_process();

    // Wait for the daemon to bind.
    let mut reachable = false;
    for _ in 0..50 {
      if manager.is_running().await {
        reachable = true;
        break;
      }
      tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(reachable, "daemon did not become reachable");

    // Query status.
    let status = manager.status().await.unwrap();
    assert!(status.running);

    // Stop the daemon.
    manager.stop().await.unwrap();
    handle.abort();
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn test_in_process_daemon_list_and_cancel_jobs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("jobs.sock");
    let manager = DaemonManager::with_socket_path(&path);
    let (handle, state) = manager.auto_spawn_in_process();

    // Wait for the daemon to bind.
    for _ in 0..50 {
      if manager.is_running().await {
        break;
      }
      tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Create a job via IPC.
    let client = SocketClient::new(&path);
    let resp = client
      .send(&Request::CreateJob {
        kind: "clone".to_string(),
        description: "clone apmw".to_string(),
      })
      .await
      .unwrap();
    let id = match resp {
      Response::JobCreated { id } => id,
      _ => panic!("expected JobCreated"),
    };

    // List jobs via the manager.
    let jobs = manager.list_jobs().await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, id);

    // Cancel the job.
    let status = manager.cancel_job(&id).await.unwrap();
    assert_eq!(status, JobStatus::Cancelled);

    // Verify via shared state.
    let info = state.jobs.get_job_info(&id).await.unwrap();
    assert_eq!(info.status, JobStatus::Cancelled);

    handle.abort();
  }

  #[tokio::test]
  async fn test_daemon_not_running_is_running_false() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("missing.sock");
    let manager = DaemonManager::with_socket_path(&path);
    assert!(!manager.is_running().await);
    let status = manager.status().await.unwrap();
    assert!(!status.running);
  }
}
