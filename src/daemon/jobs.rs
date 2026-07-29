//! Background job manager for apmw.
//!
//! Tracks background operations (clone, scan, index) with thread-safe state.
//! Job IDs are short random hex strings generated from a timestamp + counter
//! so no external `rand` dependency is required.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn};

/// A newtype around a job identifier (short hex string).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub String);

impl JobId {
  /// Create a `JobId` from a raw string.
  pub fn new(s: impl Into<String>) -> Self {
    Self(s.into())
  }

  /// Return the inner string slice.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl fmt::Display for JobId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::str::FromStr for JobId {
  type Err = std::convert::Infallible;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(Self(s.to_string()))
  }
}

/// Lifecycle status of a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
  /// Job has been created but not yet started.
  Pending,
  /// Job is currently running.
  Running,
  /// Job completed successfully.
  Completed,
  /// Job failed.
  Failed,
  /// Job was cancelled by the user.
  Cancelled,
}

impl fmt::Display for JobStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      JobStatus::Pending => write!(f, "pending"),
      JobStatus::Running => write!(f, "running"),
      JobStatus::Completed => write!(f, "completed"),
      JobStatus::Failed => write!(f, "failed"),
      JobStatus::Cancelled => write!(f, "cancelled"),
    }
  }
}

/// The kind of background job (clone, scan, index, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobKind(pub String);

impl fmt::Display for JobKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Information about a single background job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
  /// Unique job identifier.
  pub id: JobId,
  /// Kind of job (e.g. "clone", "scan", "index").
  pub kind: JobKind,
  /// Current status.
  pub status: JobStatus,
  /// Human-readable description of the job.
  pub description: String,
  /// Epoch seconds when the job was created.
  pub created_at: u64,
  /// Epoch seconds when the job reached a terminal state, if any.
  pub finished_at: Option<u64>,
  /// Error message if the job failed.
  pub error: Option<String>,
}

/// Thread-safe background job manager.
pub struct JobManager {
  jobs: Mutex<HashMap<JobId, JobInfo>>,
  counter: AtomicU64,
}

impl JobManager {
  /// Create a new empty `JobManager`.
  pub fn new() -> Self {
    Self {
      jobs: Mutex::new(HashMap::new()),
      counter: AtomicU64::new(0),
    }
  }

  /// Generate a new unique job id.
  ///
  /// The id is a short hex string derived from the current epoch milliseconds
  /// and a monotonic counter. This avoids pulling in `rand` as a direct dep.
  fn generate_id(&self) -> JobId {
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_millis() as u64)
      .unwrap_or(0);
    let n = self.counter.fetch_add(1, Ordering::Relaxed);
    // 12 hex chars: 6 from timestamp, 6 from counter
    JobId(format!("{now:06x}{n:06x}"))
  }

  /// Create a new job in the [`JobStatus::Pending`] state and return its id.
  pub async fn create_job(&self, kind: impl Into<String>, description: impl Into<String>) -> JobId {
    let id = self.generate_id();
    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_secs())
      .unwrap_or(0);
    let info = JobInfo {
      id: id.clone(),
      kind: JobKind(kind.into()),
      status: JobStatus::Pending,
      description: description.into(),
      created_at: now,
      finished_at: None,
      error: None,
    };
    let mut jobs = self.jobs.lock().await;
    jobs.insert(id.clone(), info);
    info!(job_id = %id, "job created");
    id
  }

  /// Transition a job to the [`JobStatus::Running`] state.
  pub async fn start_job(&self, id: &JobId) -> Result<(), String> {
    let mut jobs = self.jobs.lock().await;
    match jobs.get_mut(id) {
      Some(info) => {
        info.status = JobStatus::Running;
        info!(job_id = %id, "job started");
        Ok(())
      }
      None => Err(format!("job not found: {id}")),
    }
  }

  /// Mark a job as completed.
  pub async fn complete_job(&self, id: &JobId) -> Result<(), String> {
    let mut jobs = self.jobs.lock().await;
    match jobs.get_mut(id) {
      Some(info) => {
        info.status = JobStatus::Completed;
        info.finished_at = Some(now_secs());
        info!(job_id = %id, "job completed");
        Ok(())
      }
      None => Err(format!("job not found: {id}")),
    }
  }

  /// Mark a job as failed with an error message.
  pub async fn fail_job(&self, id: &JobId, error: impl Into<String>) -> Result<(), String> {
    let mut jobs = self.jobs.lock().await;
    match jobs.get_mut(id) {
      Some(info) => {
        info.status = JobStatus::Failed;
        info.error = Some(error.into());
        info.finished_at = Some(now_secs());
        warn!(job_id = %id, "job failed");
        Ok(())
      }
      None => Err(format!("job not found: {id}")),
    }
  }

  /// Cancel a job. Returns an error if the job does not exist or is already in
  /// a terminal state.
  pub async fn cancel_job(&self, id: &JobId) -> Result<(), String> {
    let mut jobs = self.jobs.lock().await;
    match jobs.get_mut(id) {
      Some(info) => {
        if info.status.is_terminal() {
          return Err(format!("job {id} is already {}", info.status));
        }
        info.status = JobStatus::Cancelled;
        info.finished_at = Some(now_secs());
        info!(job_id = %id, "job cancelled");
        Ok(())
      }
      None => Err(format!("job not found: {id}")),
    }
  }

  /// Get the status of a single job.
  pub async fn get_job_status(&self, id: &JobId) -> Option<JobStatus> {
    let jobs = self.jobs.lock().await;
    jobs.get(id).map(|i| i.status)
  }

  /// Get full info for a single job.
  pub async fn get_job_info(&self, id: &JobId) -> Option<JobInfo> {
    let jobs = self.jobs.lock().await;
    jobs.get(id).cloned()
  }

  /// List all known jobs (newest first by created_at).
  pub async fn list_jobs(&self) -> Vec<JobInfo> {
    let jobs = self.jobs.lock().await;
    let mut all: Vec<JobInfo> = jobs.values().cloned().collect();
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all
  }

  /// Remove all jobs in a terminal state.
  pub async fn prune_finished(&self) -> usize {
    let mut jobs = self.jobs.lock().await;
    let before = jobs.len();
    jobs.retain(|_, info| !info.status.is_terminal());
    let removed = before - jobs.len();
    if removed > 0 {
      info!(removed, "pruned finished jobs");
    }
    removed
  }
}

impl Default for JobManager {
  fn default() -> Self {
    Self::new()
  }
}

impl JobStatus {
  /// Returns `true` if the status is terminal (no further transitions).
  pub fn is_terminal(&self) -> bool {
    matches!(
      self,
      JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    )
  }
}

fn now_secs() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_create_job() {
    let mgr = JobManager::new();
    let id = mgr.create_job("clone", "clone apmw repo").await;
    let info = mgr.get_job_info(&id).await;
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.kind.0, "clone");
    assert_eq!(info.description, "clone apmw repo");
    assert_eq!(info.status, JobStatus::Pending);
    assert!(info.error.is_none());
    assert!(info.finished_at.is_none());
  }

  #[tokio::test]
  async fn test_start_and_complete_job() {
    let mgr = JobManager::new();
    let id = mgr.create_job("scan", "scan current project").await;
    mgr.start_job(&id).await.unwrap();
    assert_eq!(mgr.get_job_status(&id).await, Some(JobStatus::Running));
    mgr.complete_job(&id).await.unwrap();
    assert_eq!(mgr.get_job_status(&id).await, Some(JobStatus::Completed));
    let info = mgr.get_job_info(&id).await.unwrap();
    assert!(info.finished_at.is_some());
  }

  #[tokio::test]
  async fn test_fail_job() {
    let mgr = JobManager::new();
    let id = mgr.create_job("index", "index repo").await;
    mgr.start_job(&id).await.unwrap();
    mgr.fail_job(&id, "disk full").await.unwrap();
    let info = mgr.get_job_info(&id).await.unwrap();
    assert_eq!(info.status, JobStatus::Failed);
    assert_eq!(info.error.as_deref(), Some("disk full"));
  }

  #[tokio::test]
  async fn test_cancel_job() {
    let mgr = JobManager::new();
    let id = mgr.create_job("clone", "clone repo").await;
    mgr.cancel_job(&id).await.unwrap();
    let info = mgr.get_job_info(&id).await.unwrap();
    assert_eq!(info.status, JobStatus::Cancelled);
  }

  #[tokio::test]
  async fn test_cancel_terminal_job_errors() {
    let mgr = JobManager::new();
    let id = mgr.create_job("clone", "clone repo").await;
    mgr.complete_job(&id).await.unwrap();
    let res = mgr.cancel_job(&id).await;
    assert!(res.is_err());
  }

  #[tokio::test]
  async fn test_cancel_unknown_job_errors() {
    let mgr = JobManager::new();
    let res = mgr.cancel_job(&JobId("deadbeef".to_string())).await;
    assert!(res.is_err());
  }

  #[tokio::test]
  async fn test_list_jobs() {
    let mgr = JobManager::new();
    let id1 = mgr.create_job("clone", "a").await;
    let id2 = mgr.create_job("scan", "b").await;
    let list = mgr.list_jobs().await;
    assert_eq!(list.len(), 2);
    // newest first — id2 was created after id1
    assert_eq!(list[0].id, id2);
    assert_eq!(list[1].id, id1);
  }

  #[tokio::test]
  async fn test_prune_finished() {
    let mgr = JobManager::new();
    let id1 = mgr.create_job("clone", "a").await;
    let _id2 = mgr.create_job("scan", "b").await;
    mgr.complete_job(&id1).await.unwrap();
    let removed = mgr.prune_finished().await;
    assert_eq!(removed, 1);
    let list = mgr.list_jobs().await;
    assert_eq!(list.len(), 1);
  }

  #[tokio::test]
  async fn test_job_id_uniqueness() {
    let mgr = JobManager::new();
    let id1 = mgr.create_job("clone", "a").await;
    let id2 = mgr.create_job("clone", "b").await;
    assert_ne!(id1, id2);
  }

  #[test]
  fn test_job_status_display() {
    assert_eq!(JobStatus::Pending.to_string(), "pending");
    assert_eq!(JobStatus::Running.to_string(), "running");
    assert_eq!(JobStatus::Completed.to_string(), "completed");
    assert_eq!(JobStatus::Failed.to_string(), "failed");
    assert_eq!(JobStatus::Cancelled.to_string(), "cancelled");
  }

  #[test]
  fn test_job_status_is_terminal() {
    assert!(!JobStatus::Pending.is_terminal());
    assert!(!JobStatus::Running.is_terminal());
    assert!(JobStatus::Completed.is_terminal());
    assert!(JobStatus::Failed.is_terminal());
    assert!(JobStatus::Cancelled.is_terminal());
  }

  #[test]
  fn test_job_id_display() {
    let id = JobId("abc123".to_string());
    assert_eq!(id.to_string(), "abc123");
    assert_eq!(id.as_str(), "abc123");
  }
}
