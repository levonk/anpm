//! Non-blocking HTTP sender for telemetry events.
//!
//! The sender is fire-and-forget: it spawns a tokio task to POST the event
//! to the configured endpoint and never blocks the main operation. If the
//! endpoint is unreachable, the event is silently dropped.
//!
//! A trait-based design ([`TelemetrySender`]) allows tests to mock the sender
//! without making real HTTP requests.

use std::sync::{Arc, Mutex};

use tracing::debug;

use super::event::TelemetryEvent;

/// A trait for sending telemetry events.
///
/// Implementations must be non-blocking: the [`send`](TelemetrySender::send)
/// method should return immediately and perform any I/O asynchronously.
/// Failures must be silently dropped (logged at `debug` level, never `error`).
pub trait TelemetrySender: Send + Sync {
  /// Send a telemetry event.
  ///
  /// This method must not block the caller. HTTP implementations should
  /// spawn a background task and return immediately.
  fn send(&self, event: &TelemetryEvent);
}

/// HTTP-based telemetry sender using `reqwest`.
///
/// Spawns a tokio task to POST the event as JSON to the configured endpoint.
/// The send is fire-and-forget: the result is never awaited by the caller,
/// and failures are silently dropped (logged at `debug` level).
#[derive(Debug, Clone)]
pub struct HttpSender {
  endpoint: String,
}

impl HttpSender {
  /// Create a new HTTP sender targeting the given endpoint URL.
  pub fn new(endpoint: impl Into<String>) -> Self {
    HttpSender {
      endpoint: endpoint.into(),
    }
  }

  /// Returns the configured endpoint URL.
  pub fn endpoint(&self) -> &str {
    &self.endpoint
  }
}

impl TelemetrySender for HttpSender {
  fn send(&self, event: &TelemetryEvent) {
    let endpoint = self.endpoint.clone();
    let event = event.clone();

    // Attempt to spawn a non-blocking task. If no tokio runtime is available
    // (e.g. called outside an async context), silently drop the event.
    let spawn_result = tokio::runtime::Handle::try_current().map(|handle| {
      handle.spawn(async move {
        match send_event(&endpoint, &event).await {
          Ok(()) => debug!(
            endpoint = %endpoint,
            command = %event.command,
            "Telemetry event sent"
          ),
          Err(e) => debug!(
            endpoint = %endpoint,
            error = %e,
            "Telemetry send failed (silently dropped)"
          ),
        }
      });
    });

    if let Err(e) = spawn_result {
      debug!(
        error = %e,
        "Telemetry dropped: no tokio runtime available"
      );
    }
  }
}

/// Perform the actual HTTP POST of a telemetry event.
async fn send_event(endpoint: &str, event: &TelemetryEvent) -> Result<(), String> {
  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(5))
    .build()
    .map_err(|e| format!("failed to build HTTP client: {e}"))?;

  let response = client
    .post(endpoint)
    .json(event)
    .send()
    .await
    .map_err(|e| format!("HTTP request failed: {e}"))?;

  if !response.status().is_success() {
    return Err(format!(
      "telemetry endpoint returned HTTP {}",
      response.status()
    ));
  }

  Ok(())
}

/// A mock sender for testing that records sent events without making HTTP requests.
#[derive(Debug, Default)]
pub struct MockSender {
  events: Mutex<Vec<TelemetryEvent>>,
}

impl MockSender {
  /// Create a new mock sender.
  pub fn new() -> Self {
    MockSender::default()
  }

  /// Returns a clone of all events that were sent.
  pub fn events(&self) -> Vec<TelemetryEvent> {
    self.events.lock().unwrap().clone()
  }

  /// Returns the number of events that were sent.
  pub fn event_count(&self) -> usize {
    self.events.lock().unwrap().len()
  }
}

impl TelemetrySender for MockSender {
  fn send(&self, event: &TelemetryEvent) {
    debug!(command = %event.command, "Mock sender recorded telemetry event");
    self.events.lock().unwrap().push(event.clone());
  }
}

/// A sender that does nothing (no-op). Useful as a default when telemetry is disabled.
#[derive(Debug, Clone, Default)]
pub struct NoopSender;

impl TelemetrySender for NoopSender {
  fn send(&self, _event: &TelemetryEvent) {
    // No-op: telemetry is disabled.
  }
}

/// Wrap a sender in an [`Arc`] for shared ownership.
pub fn shared_sender<S: TelemetrySender + 'static>(sender: S) -> Arc<dyn TelemetrySender> {
  Arc::new(sender)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::telemetry::event::{
    ErrorCategory, Outcome, TelemetryCommand, TelemetryEvent, TelemetryTerminalType,
  };

  fn sample_event() -> TelemetryEvent {
    TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Interactive)
      .command(TelemetryCommand::Add)
      .manager("pnpm")
      .success()
      .duration_ms(100)
      .build()
      .unwrap()
  }

  #[test]
  fn test_mock_sender_records_events() {
    let sender = MockSender::new();
    let event = sample_event();
    sender.send(&event);
    assert_eq!(sender.event_count(), 1);
    assert_eq!(sender.events()[0], event);
  }

  #[test]
  fn test_mock_sender_multiple_events() {
    let sender = MockSender::new();
    let event1 = sample_event();
    let event2 = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::NonInteractive)
      .command(TelemetryCommand::Detect)
      .failure(ErrorCategory::NotFound)
      .duration_ms(50)
      .build()
      .unwrap();
    sender.send(&event1);
    sender.send(&event2);
    assert_eq!(sender.event_count(), 2);
    assert_eq!(sender.events()[0], event1);
    assert_eq!(sender.events()[1], event2);
  }

  #[test]
  fn test_noop_sender_does_nothing() {
    let sender = NoopSender;
    let event = sample_event();
    sender.send(&event);
    // No way to verify no-op directly, but it should not panic or block.
  }

  #[test]
  fn test_http_sender_endpoint() {
    let sender = HttpSender::new("https://example.com/v1/event");
    assert_eq!(sender.endpoint(), "https://example.com/v1/event");
  }

  #[test]
  fn test_http_sender_send_without_runtime_does_not_panic() {
    // This test verifies that calling send without a tokio runtime
    // does not panic — it should silently drop the event.
    let sender = HttpSender::new("https://example.com/v1/event");
    let event = sample_event();
    // This should not panic even if no runtime is available.
    sender.send(&event);
  }

  #[tokio::test]
  async fn test_http_sender_send_with_runtime_spawns_task() {
    let sender = HttpSender::new("https://nonexistent.invalid/v1/event");
    let event = sample_event();
    // This should spawn a task and return immediately.
    sender.send(&event);
    // Give the spawned task a moment to attempt (and fail) the request.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // The task should have failed silently (logged at debug level).
  }

  #[test]
  fn test_shared_sender() {
    let sender = shared_sender(MockSender::new());
    let event = sample_event();
    sender.send(&event);
    // Verify it's usable as a trait object.
  }

  #[test]
  fn test_telemetry_sender_trait_object() {
    let sender: Arc<dyn TelemetrySender> = Arc::new(MockSender::new());
    let event = sample_event();
    sender.send(&event);
    // Verify it's usable as a trait object via Arc.
  }

  #[test]
  fn test_outcome_and_error_category_in_event() {
    let event = TelemetryEvent::builder()
      .terminal_type(TelemetryTerminalType::Login)
      .command(TelemetryCommand::Clone)
      .failure(ErrorCategory::Network)
      .duration_ms(5000)
      .build()
      .unwrap();

    assert_eq!(event.outcome, Outcome::Failure);
    assert_eq!(event.error_category, Some(ErrorCategory::Network));
  }
}
