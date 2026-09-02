//! MCP (Model Context Protocol) server with stdio transport.
//!
//! Implements a minimal MCP server that AI agents (Claude Code, Codex,
//! OpenCode) connect to for `add`/`detect`/`scan`/`clone` operations. The
//! server speaks JSON-RPC 2.0 over stdio using **line-delimited** messages
//! (one JSON object per line). This variant was chosen over the
//! `Content-Length` header framing from the MCP spec because it is simpler
//! to parse, easier to debug, and sufficient for local stdio transport where
//! messages are not arbitrarily large. The protocol is abstracted behind a
//! [`Transport`] trait so the framing can be swapped without touching the
//! server logic.
//!
//! # Protocol flow
//!
//! 1. **Initialize** — the client sends an `initialize` request; the server
//!    responds with its protocol version, capabilities, and server info.
//! 2. **tools/list** — the client requests the list of available tools; the
//!    server returns each tool's name, description, and JSON input schema.
//! 3. **tools/call** — the client invokes a tool by name with JSON arguments;
//!    the server executes the tool and returns the result as MCP content.
//! 4. **Notifications** — messages without an `id` (e.g. `notifications/
//!    initialized`) are acknowledged silently (no response).
//!
//! # Concurrency
//!
//! Tool calls are executed as spawned tokio tasks. A semaphore limits the
//! number of concurrently running tools (default 8) so a burst of requests
//! does not exhaust system resources. Each tool implementation is
//! `Send + Sync` so it can be shared across tasks safely.
//!
//! # Tools exposed
//!
//! | Tool                | Delegates to                                    |
//! |---------------------|-------------------------------------------------|
//! | `apmw_add`          | [`AddEngine`]                                   |
//! | `apmw_detect`       | [`DetectionEngine`]                             |
//! | `apmw_scan`         | [`ScanOrchestrator`]                            |
//! | `apmw_scan_container` | [`scan_image`]                               |
//! | `apmw_clone`        | [`CloneEngine`]                                 |
//! | `apmw_status`       | [`crate::version()`] + daemon status            |
//! | `apmw_list_jobs`    | [`crate::daemon::JobManager`]                   |

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{Mutex, Semaphore};
use tracing::{error, info, warn};

use crate::clone::CloneEngine;
use crate::containers::{scan_image, TokioExecutor};
use crate::daemon::DaemonManager;
use crate::detect::DetectionEngine;
use crate::error::{ApmwError, Result};
use crate::install::{AddEngine, AddEngineConfig};
use crate::security::{PackageScanRequest, ScanConfig, ScanOrchestrator};
use crate::version::MinAgeDaysConfig;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The MCP protocol version this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// The server name reported during the initialize handshake.
pub const SERVER_NAME: &str = "apmw";

/// Default maximum number of concurrently executing tool calls.
pub const DEFAULT_MAX_CONCURRENT_TOOLS: usize = 8;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 message types
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 request id (integer or string, never null per spec).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
  /// Numeric id.
  Number(i64),
  /// String id.
  String(String),
}

impl fmt::Display for RequestId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RequestId::Number(n) => write!(f, "{n}"),
      RequestId::String(s) => write!(f, "{s}"),
    }
  }
}

/// A JSON-RPC 2.0 request or notification.
///
/// A message without an `id` is a notification (no response expected). A
/// message with an `id` is a request that must be answered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
  /// Always `"2.0"`.
  pub jsonrpc: String,
  /// The method name (e.g. `"initialize"`, `"tools/list"`).
  pub method: String,
  /// Optional parameters (object or array).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<Value>,
  /// The request id. `None` for notifications.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<RequestId>,
}

impl JsonRpcMessage {
  /// Returns `true` if this message is a notification (no `id`).
  pub fn is_notification(&self) -> bool {
    self.id.is_none()
  }

  /// Returns `true` if this message is a request (has an `id`).
  pub fn is_request(&self) -> bool {
    self.id.is_some()
  }
}

/// A JSON-RPC 2.0 successful response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
  /// Always `"2.0"`.
  pub jsonrpc: String,
  /// The request id this response corresponds to.
  pub id: RequestId,
  /// The result payload.
  pub result: Value,
}

/// A JSON-RPC 2.0 error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
  /// Always `"2.0"`.
  pub jsonrpc: String,
  /// The request id this error corresponds to.
  pub id: RequestId,
  /// The error object.
  pub error: RpcError,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
  /// Numeric error code.
  pub code: i32,
  /// Human-readable error message.
  pub message: String,
  /// Optional additional data.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
}

impl RpcError {
  /// Creates a new RPC error with the given code and message.
  pub fn new(code: i32, message: impl Into<String>) -> Self {
    RpcError {
      code,
      message: message.into(),
      data: None,
    }
  }

  /// Attaches additional data to the error.
  pub fn with_data(mut self, data: Value) -> Self {
    self.data = Some(data);
    self
  }
}

/// Standard JSON-RPC error codes.
pub mod error_codes {
  /// Parse error — invalid JSON received.
  pub const PARSE_ERROR: i32 = -32700;
  /// Invalid request — the JSON is not a valid request object.
  pub const INVALID_REQUEST: i32 = -32600;
  /// Method not found — the method does not exist or is not available.
  pub const METHOD_NOT_FOUND: i32 = -32601;
  /// Invalid params — invalid method parameters.
  pub const INVALID_PARAMS: i32 = -32602;
  /// Internal error — internal JSON-RPC error.
  pub const INTERNAL_ERROR: i32 = -32603;
  /// Server error (application-defined, -32000 to -32099).
  pub const SERVER_ERROR: i32 = -32000;
}

// ---------------------------------------------------------------------------
// MCP protocol types
// ---------------------------------------------------------------------------

/// The server's capabilities, reported during the initialize handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
  /// Tools capability — this server exposes tools.
  pub tools: ToolsCapability,
}

/// The tools capability object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
  /// Whether the server supports `notifications/tools/list_changed`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub list_changed: Option<bool>,
}

/// Server info reported during the initialize handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
  /// The server name.
  pub name: String,
  /// The server version.
  pub version: String,
}

/// The result of the `initialize` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
  /// The protocol version the server speaks.
  pub protocol_version: String,
  /// The server's capabilities.
  pub capabilities: ServerCapabilities,
  /// Information about the server.
  pub server_info: ServerInfo,
}

/// A single tool definition returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
  /// The tool name (e.g. `"apmw_add"`).
  pub name: String,
  /// Human-readable description of what the tool does.
  pub description: String,
  /// The JSON Schema for the tool's input parameters.
  pub input_schema: Value,
}

/// The result of `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResult {
  /// The list of available tools.
  pub tools: Vec<ToolDefinition>,
}

/// A single content item in a tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
  /// The content type (always `"text"` for this server).
  #[serde(rename = "type")]
  pub content_type: String,
  /// The text content.
  pub text: String,
}

impl ContentItem {
  /// Creates a new text content item.
  pub fn text(text: impl Into<String>) -> Self {
    ContentItem {
      content_type: "text".to_string(),
      text: text.into(),
    }
  }
}

/// The result of `tools/call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
  /// The content items returned by the tool.
  pub content: Vec<ContentItem>,
  /// Whether the tool call resulted in an error.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub is_error: Option<bool>,
}

impl ToolCallResult {
  /// Creates a successful result with a single text content item.
  pub fn success(text: impl Into<String>) -> Self {
    ToolCallResult {
      content: vec![ContentItem::text(text)],
      is_error: None,
    }
  }

  /// Creates a successful result from a JSON value (pretty-printed).
  pub fn success_json(value: &Value) -> Self {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    ToolCallResult::success(text)
  }

  /// Creates an error result with a single text content item.
  pub fn error(text: impl Into<String>) -> Self {
    ToolCallResult {
      content: vec![ContentItem::text(text)],
      is_error: Some(true),
    }
  }
}

// ---------------------------------------------------------------------------
// Transport trait
// ---------------------------------------------------------------------------

/// Abstracts the transport layer for MCP messages.
///
/// The default implementation is [`StdioTransport`] (line-delimited JSON over
/// stdin/stdout). The trait allows alternative transports (SSE, WebSocket) to
/// be added in the future without changing the server logic.
pub trait Transport: Send {
  /// Read the next raw JSON-RPC message line. Returns `None` at EOF.
  fn read_message(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = std::result::Result<Option<String>, std::io::Error>> + Send + '_>>;

  /// Write a raw JSON-RPC response line.
  fn write_message(
    &mut self,
    line: &str,
  ) -> Pin<Box<dyn Future<Output = std::result::Result<(), std::io::Error>> + Send + '_>>;
}

/// Line-delimited JSON transport over tokio stdin/stdout.
///
/// Each message is a single line of UTF-8 JSON terminated by `\n`. Responses
/// are written as a single line followed by `\n` and flushed immediately.
pub struct StdioTransport {
  reader: BufReader<tokio::io::Stdin>,
  writer: BufWriter<tokio::io::Stdout>,
}

impl StdioTransport {
  /// Create a new stdio transport.
  pub fn new() -> Self {
    StdioTransport {
      reader: BufReader::new(tokio::io::stdin()),
      writer: BufWriter::new(tokio::io::stdout()),
    }
  }
}

impl Default for StdioTransport {
  fn default() -> Self {
    Self::new()
  }
}

impl Transport for StdioTransport {
  fn read_message(
    &mut self,
  ) -> Pin<Box<dyn Future<Output = std::result::Result<Option<String>, std::io::Error>> + Send + '_>>
  {
    Box::pin(async move {
      let mut line = String::new();
      let n = self.reader.read_line(&mut line).await?;
      if n == 0 {
        return Ok(None); // EOF
      }
      let trimmed = line.trim_end_matches(['\r', '\n']);
      if trimmed.is_empty() {
        return Ok(Some(String::new()));
      }
      Ok(Some(trimmed.to_string()))
    })
  }

  fn write_message(
    &mut self,
    line: &str,
  ) -> Pin<Box<dyn Future<Output = std::result::Result<(), std::io::Error>> + Send + '_>> {
    let line = line.to_string();
    Box::pin(async move {
      self.writer.write_all(line.as_bytes()).await?;
      self.writer.write_all(b"\n").await?;
      self.writer.flush().await?;
      Ok(())
    })
  }
}

// ---------------------------------------------------------------------------
// Tool trait and registry
// ---------------------------------------------------------------------------

/// A boxed future returned by the [`Tool`] trait.
type ToolFuture<'a> = Pin<Box<dyn Future<Output = ToolCallResult> + Send + 'a>>;

/// A tool that can be invoked via `tools/call`.
///
/// Each tool receives its arguments as a [`serde_json::Value`] (the
/// `arguments` field from the `tools/call` request) and returns a
/// [`ToolCallResult`] containing MCP-compatible content.
pub trait Tool: Send + Sync {
  /// The tool name (e.g. `"apmw_add"`).
  fn name(&self) -> &str;

  /// A human-readable description of what the tool does.
  fn description(&self) -> &str;

  /// The JSON Schema for the tool's input parameters.
  fn input_schema(&self) -> Value;

  /// Execute the tool with the given arguments.
  fn call(&self, arguments: &Value) -> ToolFuture<'_>;
}

/// A registry of tools keyed by name.
pub struct ToolRegistry {
  tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
  /// Create a new empty registry.
  pub fn new() -> Self {
    ToolRegistry {
      tools: HashMap::new(),
    }
  }

  /// Register a tool.
  pub fn register(&mut self, tool: Arc<dyn Tool>) {
    let name = tool.name().to_string();
    info!(tool = %name, "registered MCP tool");
    self.tools.insert(name, tool);
  }

  /// Look up a tool by name.
  pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
    self.tools.get(name)
  }

  /// Return all registered tools as a list of definitions.
  pub fn definitions(&self) -> Vec<ToolDefinition> {
    self
      .tools
      .values()
      .map(|t| ToolDefinition {
        name: t.name().to_string(),
        description: t.description().to_string(),
        input_schema: t.input_schema(),
      })
      .collect()
  }

  /// Returns the number of registered tools.
  pub fn len(&self) -> usize {
    self.tools.len()
  }

  /// Returns `true` if no tools are registered.
  pub fn is_empty(&self) -> bool {
    self.tools.is_empty()
  }
}

impl Default for ToolRegistry {
  fn default() -> Self {
    Self::new()
  }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

/// Helper: extract a string field from JSON arguments, falling back to a
/// default.
fn get_str(args: &Value, key: &str, default: &str) -> String {
  args
    .get(key)
    .and_then(|v| v.as_str())
    .unwrap_or(default)
    .to_string()
}

/// Helper: extract an optional string field from JSON arguments.
fn get_opt_str(args: &Value, key: &str) -> Option<String> {
  args
    .get(key)
    .and_then(|v| v.as_str())
    .map(|s| s.to_string())
}

/// Helper: extract a boolean field from JSON arguments.
fn get_bool(args: &Value, key: &str, default: bool) -> bool {
  args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

/// Helper: extract a string field that is required, returning an error result
/// if missing.
fn require_str(args: &Value, key: &str) -> std::result::Result<String, ToolCallResult> {
  args
    .get(key)
    .and_then(|v| v.as_str())
    .map(|s| s.to_string())
    .ok_or_else(|| ToolCallResult::error(format!("missing required parameter: '{key}'")))
}

// --- apmw_add ---

/// The `apmw_add` tool — delegates to [`AddEngine`].
pub struct AddTool;

impl AddTool {
  /// Create a new instance.
  pub fn new() -> Self {
    AddTool
  }
}

impl Default for AddTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for AddTool {
  fn name(&self) -> &str {
    "apmw_add"
  }

  fn description(&self) -> &str {
    "Add a package or tool using apmw's install-on-use flow. Detects the \
     package manager, scans PATH, resolves versions, runs security scanning, \
     and executes the add command."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {
        "package": {
          "type": "string",
          "description": "The package name or specifier to install."
        },
        "project_dir": {
          "type": "string",
          "description": "The project directory to operate in. Defaults to the current directory."
        },
        "dev": {
          "type": "boolean",
          "description": "Install as a development/build-time dependency.",
          "default": false
        },
        "dry_run": {
          "type": "boolean",
          "description": "Show what would happen without making changes.",
          "default": false
        },
        "no_scan": {
          "type": "boolean",
          "description": "Skip security scanning.",
          "default": false
        },
        "manager": {
          "type": "string",
          "description": "Override auto-detection and force a specific package manager."
        },
        "version_spec": {
          "type": "string",
          "description": "The requested version spec (e.g. \"4\", \"^1.2\").",
          "default": ""
        }
      },
      "required": ["package"]
    })
  }

  fn call(&self, arguments: &Value) -> ToolFuture<'_> {
    let args = arguments.clone();
    Box::pin(async move {
      let package = match require_str(&args, "package") {
        Ok(p) => p,
        Err(e) => return e,
      };
      let project_dir = get_str(&args, "project_dir", ".");
      let dev = get_bool(&args, "dev", false);
      let dry_run = get_bool(&args, "dry_run", false);
      let no_scan = get_bool(&args, "no_scan", false);
      let manager_override = get_opt_str(&args, "manager");
      let version_spec = get_str(&args, "version_spec", "");

      info!(tool = "apmw_add", package = %package, "MCP tool call");

      let config = AddEngineConfig {
        dev,
        dry_run,
        no_scan,
        scan_only: false,
        manager_override,
        version_spec,
        use_devbox: true,
        on_risk: crate::security::OnRiskMode::Warn,
        min_age: MinAgeDaysConfig::default(),
      };

      let engine = AddEngine::new();
      match engine
        .run(&package, &PathBuf::from(&project_dir), &config)
        .await
      {
        Ok(result) => {
          let value = serde_json::to_value(&result).unwrap_or_else(|_| {
            serde_json::json!({
              "package": package,
              "status": "unknown",
            })
          });
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_add", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_add failed: {e}"))
        }
      }
    })
  }
}

// --- apmw_detect ---

/// The `apmw_detect` tool — delegates to [`DetectionEngine`].
pub struct DetectTool;

impl DetectTool {
  /// Create a new instance.
  pub fn new() -> Self {
    DetectTool
  }
}

impl Default for DetectTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for DetectTool {
  fn name(&self) -> &str {
    "apmw_detect"
  }

  fn description(&self) -> &str {
    "Detect the package manager(s) for a project directory by scanning for \
     lockfiles, config files, and directory markers."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {
        "project_dir": {
          "type": "string",
          "description": "The project directory to scan. Defaults to the current directory."
        }
      }
    })
  }

  fn call(&self, arguments: &Value) -> ToolFuture<'_> {
    let args = arguments.clone();
    Box::pin(async move {
      let project_dir = get_str(&args, "project_dir", ".");

      info!(tool = "apmw_detect", dir = %project_dir, "MCP tool call");

      let engine = DetectionEngine::new();
      match engine.detect(&PathBuf::from(&project_dir)) {
        Ok(results) => {
          let value = serde_json::to_value(&results).unwrap_or_else(|_| serde_json::json!([]));
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_detect", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_detect failed: {e}"))
        }
      }
    })
  }
}

// --- apmw_scan ---

/// The `apmw_scan` tool — delegates to [`ScanOrchestrator`].
pub struct ScanTool;

impl ScanTool {
  /// Create a new instance.
  pub fn new() -> Self {
    ScanTool
  }
}

impl Default for ScanTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for ScanTool {
  fn name(&self) -> &str {
    "apmw_scan"
  }

  fn description(&self) -> &str {
    "Run a two-phase security scan over a batch of packages. Scans all \
     packages first, then decides an install action per package based on the \
     on-risk mode."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {
        "packages": {
          "type": "array",
          "description": "The packages to scan.",
          "items": {
            "type": "object",
            "properties": {
              "name": { "type": "string", "description": "The package name." },
              "dir": { "type": "string", "description": "The local directory to scan." }
            },
            "required": ["name", "dir"]
          }
        },
        "no_scan": {
          "type": "boolean",
          "description": "Skip all scanning and mark every package as safe.",
          "default": false
        },
        "scan_only": {
          "type": "boolean",
          "description": "Scan but do not install (advisory only).",
          "default": false
        }
      },
      "required": ["packages"]
    })
  }

  fn call(&self, arguments: &Value) -> ToolFuture<'_> {
    let args = arguments.clone();
    Box::pin(async move {
      let packages_arr = match args.get("packages").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
          return ToolCallResult::error("missing required parameter: 'packages' (must be an array)")
        }
      };

      let no_scan = get_bool(&args, "no_scan", false);
      let scan_only = get_bool(&args, "scan_only", false);

      info!(
        tool = "apmw_scan",
        package_count = packages_arr.len(),
        "MCP tool call"
      );

      let mut packages: Vec<PackageScanRequest> = Vec::new();
      for item in &packages_arr {
        let name = match item.get("name").and_then(|v| v.as_str()) {
          Some(n) => n.to_string(),
          None => return ToolCallResult::error("each package must have a 'name' field"),
        };
        let dir = match item.get("dir").and_then(|v| v.as_str()) {
          Some(d) => d.to_string(),
          None => return ToolCallResult::error("each package must have a 'dir' field"),
        };
        packages.push(PackageScanRequest::new(name, PathBuf::from(dir)));
      }

      let config = ScanConfig::default()
        .with_no_scan(no_scan)
        .with_scan_only(scan_only);
      let mut orchestrator = ScanOrchestrator::new(config);

      match orchestrator.scan_all(&packages) {
        Ok(report) => {
          let value = serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({}));
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_scan", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_scan failed: {e}"))
        }
      }
    })
  }
}

// --- apmw_scan_container ---

/// The `apmw_scan_container` tool — delegates to [`scan_image`].
pub struct ScanContainerTool;

impl ScanContainerTool {
  /// Create a new instance.
  pub fn new() -> Self {
    ScanContainerTool
  }
}

impl Default for ScanContainerTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for ScanContainerTool {
  fn name(&self) -> &str {
    "apmw_scan_container"
  }

  fn description(&self) -> &str {
    "Scan a container image for vulnerabilities using trivy and/or grype. \
     Aggregates results with OR semantics: any risky verdict makes the \
     overall result risky."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {
        "image": {
          "type": "string",
          "description": "The container image to scan (e.g. \"nginx:latest\")."
        }
      },
      "required": ["image"]
    })
  }

  fn call(&self, arguments: &Value) -> ToolFuture<'_> {
    let args = arguments.clone();
    Box::pin(async move {
      let image = match require_str(&args, "image") {
        Ok(i) => i,
        Err(e) => return e,
      };

      info!(tool = "apmw_scan_container", image = %image, "MCP tool call");

      let executor = TokioExecutor::new();
      match scan_image(&executor, &image).await {
        Ok(result) => {
          let value =
            serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({ "image": image }));
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_scan_container", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_scan_container failed: {e}"))
        }
      }
    })
  }
}

// --- apmw_clone ---

/// The `apmw_clone` tool — delegates to [`CloneEngine`].
pub struct CloneTool;

impl CloneTool {
  /// Create a new instance.
  pub fn new() -> Self {
    CloneTool
  }
}

impl Default for CloneTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for CloneTool {
  fn name(&self) -> &str {
    "apmw_clone"
  }

  fn description(&self) -> &str {
    "Create a historyless clone of a repository with AST indexing. Runs \
     `git clone --depth 1 --single-branch --no-tags`, writes a local \
     .gitignore, and invokes the appropriate AST indexing tool."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {
        "repo": {
          "type": "string",
          "description": "The repository URL or package name to clone."
        },
        "dest_dir": {
          "type": "string",
          "description": "The destination directory. Defaults to the apmw clone cache."
        }
      },
      "required": ["repo"]
    })
  }

  fn call(&self, arguments: &Value) -> ToolFuture<'_> {
    let args = arguments.clone();
    Box::pin(async move {
      let repo = match require_str(&args, "repo") {
        Ok(r) => r,
        Err(e) => return e,
      };
      let dest_dir = get_opt_str(&args, "dest_dir")
        .map(PathBuf::from)
        .unwrap_or_else(crate::clone::default_clone_dest);

      info!(tool = "apmw_clone", repo = %repo, "MCP tool call");

      let engine = CloneEngine::new();
      match engine.clone_repo(&repo, &dest_dir).await {
        Ok(result) => {
          let value =
            serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({ "repo": repo }));
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_clone", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_clone failed: {e}"))
        }
      }
    })
  }
}

// --- apmw_status ---

/// The `apmw_status` tool — returns version and daemon status.
pub struct StatusTool;

impl StatusTool {
  /// Create a new instance.
  pub fn new() -> Self {
    StatusTool
  }
}

impl Default for StatusTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for StatusTool {
  fn name(&self) -> &str {
    "apmw_status"
  }

  fn description(&self) -> &str {
    "Return apmw status information: version, daemon running state, and \
     daemon process id."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {}
    })
  }

  fn call(&self, _arguments: &Value) -> ToolFuture<'_> {
    Box::pin(async move {
      info!(tool = "apmw_status", "MCP tool call");

      let manager = DaemonManager::new();
      let daemon_status = manager
        .status()
        .await
        .unwrap_or(crate::daemon::DaemonStatus {
          running: false,
          pid: None,
        });

      let value = serde_json::json!({
        "version": crate::version(),
        "daemon_running": daemon_status.running,
        "daemon_pid": daemon_status.pid,
      });
      ToolCallResult::success_json(&value)
    })
  }
}

// --- apmw_list_jobs ---

/// The `apmw_list_jobs` tool — delegates to the daemon's
/// [`crate::daemon::JobManager`].
pub struct ListJobsTool;

impl ListJobsTool {
  /// Create a new instance.
  pub fn new() -> Self {
    ListJobsTool
  }
}

impl Default for ListJobsTool {
  fn default() -> Self {
    Self::new()
  }
}

impl Tool for ListJobsTool {
  fn name(&self) -> &str {
    "apmw_list_jobs"
  }

  fn description(&self) -> &str {
    "List background jobs tracked by the apmw daemon (clone, scan, index). \
     Returns an empty list if the daemon is not running."
  }

  fn input_schema(&self) -> Value {
    serde_json::json!({
      "type": "object",
      "properties": {}
    })
  }

  fn call(&self, _arguments: &Value) -> ToolFuture<'_> {
    Box::pin(async move {
      info!(tool = "apmw_list_jobs", "MCP tool call");

      let manager = DaemonManager::new();
      if !manager.is_running().await {
        return ToolCallResult::success_json(&serde_json::json!([]));
      }
      match manager.list_jobs().await {
        Ok(jobs) => {
          let value = serde_json::to_value(&jobs).unwrap_or_else(|_| serde_json::json!([]));
          ToolCallResult::success_json(&value)
        }
        Err(e) => {
          error!(tool = "apmw_list_jobs", error = %e, "tool call failed");
          ToolCallResult::error(format!("apmw_list_jobs failed: {e}"))
        }
      }
    })
  }
}

// ---------------------------------------------------------------------------
// MCP server
// ---------------------------------------------------------------------------

/// The MCP server.
///
/// Reads JSON-RPC messages from a [`Transport`], dispatches them to the
/// registered tools, and writes responses back. Tool calls are executed
/// concurrently (bounded by a semaphore).
pub struct McpServer {
  registry: Arc<ToolRegistry>,
  semaphore: Arc<Semaphore>,
  initialized: Arc<Mutex<bool>>,
}

impl McpServer {
  /// Create a new MCP server with the given tool registry and concurrency
  /// limit.
  pub fn new(registry: ToolRegistry, max_concurrent: usize) -> Self {
    McpServer {
      registry: Arc::new(registry),
      semaphore: Arc::new(Semaphore::new(max_concurrent)),
      initialized: Arc::new(Mutex::new(false)),
    }
  }

  /// Create a new MCP server with the default tool set and concurrency limit.
  pub fn with_default_tools() -> Self {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(AddTool::new()));
    registry.register(Arc::new(DetectTool::new()));
    registry.register(Arc::new(ScanTool::new()));
    registry.register(Arc::new(ScanContainerTool::new()));
    registry.register(Arc::new(CloneTool::new()));
    registry.register(Arc::new(StatusTool::new()));
    registry.register(Arc::new(ListJobsTool::new()));
    McpServer::new(registry, DEFAULT_MAX_CONCURRENT_TOOLS)
  }

  /// Create a new MCP server with a custom tool registry and the default
  /// concurrency limit.
  pub fn with_registry(registry: ToolRegistry) -> Self {
    McpServer::new(registry, DEFAULT_MAX_CONCURRENT_TOOLS)
  }

  /// Returns a reference to the tool registry.
  pub fn registry(&self) -> &ToolRegistry {
    &self.registry
  }

  /// Run the server loop over the given transport until EOF or a fatal error.
  ///
  /// Reads messages one at a time, dispatches requests, and writes responses.
  /// Notifications are processed but do not produce a response.
  pub async fn run<T: Transport>(&self, transport: &mut T) -> Result<()> {
    info!("MCP server starting (stdio transport, line-delimited JSON-RPC)");

    loop {
      let line = match transport.read_message().await? {
        Some(l) if l.is_empty() => continue,
        Some(l) => l,
        None => {
          info!("MCP server: transport EOF, shutting down");
          break;
        }
      };

      // Parse the JSON-RPC message.
      let message: JsonRpcMessage = match serde_json::from_str(&line) {
        Ok(m) => m,
        Err(e) => {
          error!(error = %e, line = %line, "failed to parse JSON-RPC message");
          let error = JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(0),
            error: RpcError::new(error_codes::PARSE_ERROR, format!("parse error: {e}")),
          };
          let json =
            serde_json::to_string(&error).map_err(|e| ApmwError::McpError(e.to_string()))?;
          transport.write_message(&json).await?;
          continue;
        }
      };

      // Notifications do not get a response.
      if message.is_notification() {
        self.handle_notification(&message).await;
        continue;
      }

      // Requests get a response.
      let response = self.handle_request(&message).await;
      let json =
        serde_json::to_string(&response).map_err(|e| ApmwError::McpError(e.to_string()))?;
      transport.write_message(&json).await?;
    }

    info!("MCP server stopped");
    Ok(())
  }

  /// Handle a notification (no response expected).
  async fn handle_notification(&self, message: &JsonRpcMessage) {
    match message.method.as_str() {
      "notifications/initialized" => {
        info!("MCP client sent initialized notification");
        *self.initialized.lock().await = true;
      }
      other => {
        debug_notification(other);
      }
    }
  }

  /// Handle a request and produce a response (success or error).
  async fn handle_request(&self, message: &JsonRpcMessage) -> McpResponse {
    let id = message.id.clone().unwrap_or(RequestId::Number(0));

    match message.method.as_str() {
      "initialize" => self.handle_initialize(&id, message).await,
      "tools/list" => self.handle_tools_list(&id).await,
      "tools/call" => self.handle_tools_call(&id, message).await,
      method => {
        warn!(method = method, "unknown MCP method");
        McpResponse::Error(JsonRpcErrorResponse {
          jsonrpc: "2.0".to_string(),
          id,
          error: RpcError::new(
            error_codes::METHOD_NOT_FOUND,
            format!("method not found: {method}"),
          ),
        })
      }
    }
  }

  /// Handle the `initialize` request.
  async fn handle_initialize(&self, id: &RequestId, _message: &JsonRpcMessage) -> McpResponse {
    info!("MCP initialize request");

    let init_result = InitializeResult {
      protocol_version: PROTOCOL_VERSION.to_string(),
      capabilities: ServerCapabilities {
        tools: ToolsCapability { list_changed: None },
      },
      server_info: ServerInfo {
        name: SERVER_NAME.to_string(),
        version: crate::version().to_string(),
      },
    };

    let value = serde_json::to_value(&init_result).unwrap_or_else(|_| serde_json::json!({}));

    McpResponse::Success(JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      id: id.clone(),
      result: value,
    })
  }

  /// Handle the `tools/list` request.
  async fn handle_tools_list(&self, id: &RequestId) -> McpResponse {
    let tools = self.registry.definitions();
    let list_result = ToolListResult { tools };
    let value =
      serde_json::to_value(&list_result).unwrap_or_else(|_| serde_json::json!({ "tools": [] }));

    McpResponse::Success(JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      id: id.clone(),
      result: value,
    })
  }

  /// Handle the `tools/call` request.
  async fn handle_tools_call(&self, id: &RequestId, message: &JsonRpcMessage) -> McpResponse {
    let params = message.params.clone().unwrap_or(Value::Null);
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

    if tool_name.is_empty() {
      return McpResponse::Error(JsonRpcErrorResponse {
        jsonrpc: "2.0".to_string(),
        id: id.clone(),
        error: RpcError::new(
          error_codes::INVALID_PARAMS,
          "missing 'name' in tools/call params",
        ),
      });
    }

    let tool = match self.registry.get(tool_name) {
      Some(t) => t.clone(),
      None => {
        warn!(tool = tool_name, "unknown tool requested");
        return McpResponse::Error(JsonRpcErrorResponse {
          jsonrpc: "2.0".to_string(),
          id: id.clone(),
          error: RpcError::new(
            error_codes::INVALID_PARAMS,
            format!("unknown tool: {tool_name}"),
          ),
        });
      }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    // Acquire a concurrency permit before executing the tool.
    let permit = match self.semaphore.acquire().await {
      Ok(p) => p,
      Err(e) => {
        error!(error = %e, "semaphore closed");
        return McpResponse::Error(JsonRpcErrorResponse {
          jsonrpc: "2.0".to_string(),
          id: id.clone(),
          error: RpcError::new(error_codes::INTERNAL_ERROR, "internal concurrency error"),
        });
      }
    };

    // Execute the tool. The permit is held for the duration of the call.
    let result = tool.call(&arguments).await;
    drop(permit);

    let value = serde_json::to_value(&result)
      .unwrap_or_else(|_| serde_json::json!({ "content": [], "isError": true }));

    McpResponse::Success(JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      id: id.clone(),
      result: value,
    })
  }
}

/// A response that can be either a success or an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResponse {
  /// A successful JSON-RPC response.
  Success(JsonRpcResponse),
  /// A JSON-RPC error response.
  Error(JsonRpcErrorResponse),
}

/// Log a notification at debug level (tracing macro requires a literal level,
/// so we use a helper).
fn debug_notification(method: &str) {
  tracing::debug!(method = method, "received notification");
}

/// Run the MCP server with the default stdio transport and default tools.
///
/// This is the entry point used by the `apmw mcp` CLI subcommand.
pub async fn run_stdio_server() -> Result<()> {
  let server = McpServer::with_default_tools();
  let mut transport = StdioTransport::new();
  server.run(&mut transport).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Cursor;
  use std::sync::Arc;
  use tempfile::TempDir;

  // --- A simple in-memory transport for testing ---

  /// An in-memory transport that reads from a string buffer and collects
  /// written lines into a Vec.
  pub struct InMemoryTransport {
    input_lines: Vec<String>,
    input_idx: usize,
    output_lines: Vec<String>,
  }

  impl InMemoryTransport {
    pub fn new(input: &str) -> Self {
      let input_lines: Vec<String> = input
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
      InMemoryTransport {
        input_lines,
        input_idx: 0,
        output_lines: Vec::new(),
      }
    }

    pub fn output(&self) -> &[String] {
      &self.output_lines
    }
  }

  impl Transport for InMemoryTransport {
    fn read_message(
      &mut self,
    ) -> Pin<
      Box<dyn Future<Output = std::result::Result<Option<String>, std::io::Error>> + Send + '_>,
    > {
      Box::pin(async move {
        if self.input_idx >= self.input_lines.len() {
          return Ok(None);
        }
        let line = self.input_lines[self.input_idx].clone();
        self.input_idx += 1;
        Ok(Some(line))
      })
    }

    fn write_message(
      &mut self,
      line: &str,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<(), std::io::Error>> + Send + '_>> {
      let line = line.to_string();
      Box::pin(async move {
        self.output_lines.push(line);
        Ok(())
      })
    }
  }

  // --- A simple echo tool for testing ---

  struct EchoTool;

  impl Tool for EchoTool {
    fn name(&self) -> &str {
      "echo"
    }
    fn description(&self) -> &str {
      "Echoes the input arguments as JSON."
    }
    fn input_schema(&self) -> Value {
      serde_json::json!({ "type": "object" })
    }
    fn call(&self, arguments: &Value) -> ToolFuture<'_> {
      let args = arguments.clone();
      Box::pin(async move { ToolCallResult::success_json(&args) })
    }
  }

  // --- JSON-RPC message tests ---

  #[test]
  fn test_request_id_display() {
    assert_eq!(RequestId::Number(42).to_string(), "42");
    assert_eq!(RequestId::String("abc".to_string()).to_string(), "abc");
  }

  #[test]
  fn test_json_rpc_message_is_notification() {
    let msg = JsonRpcMessage {
      jsonrpc: "2.0".to_string(),
      method: "notifications/initialized".to_string(),
      params: None,
      id: None,
    };
    assert!(msg.is_notification());
    assert!(!msg.is_request());
  }

  #[test]
  fn test_json_rpc_message_is_request() {
    let msg = JsonRpcMessage {
      jsonrpc: "2.0".to_string(),
      method: "initialize".to_string(),
      params: None,
      id: Some(RequestId::Number(1)),
    };
    assert!(msg.is_request());
    assert!(!msg.is_notification());
  }

  #[test]
  fn test_json_rpc_message_serialize_roundtrip() {
    let msg = JsonRpcMessage {
      jsonrpc: "2.0".to_string(),
      method: "tools/list".to_string(),
      params: Some(serde_json::json!({})),
      id: Some(RequestId::Number(1)),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: JsonRpcMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.method, "tools/list");
    assert_eq!(parsed.id, Some(RequestId::Number(1)));
  }

  #[test]
  fn test_rpc_error_new() {
    let err = RpcError::new(-32601, "method not found");
    assert_eq!(err.code, -32601);
    assert_eq!(err.message, "method not found");
    assert!(err.data.is_none());
  }

  #[test]
  fn test_rpc_error_with_data() {
    let err = RpcError::new(-32602, "bad params").with_data(serde_json::json!({"key": "val"}));
    assert_eq!(err.data, Some(serde_json::json!({"key": "val"})));
  }

  #[test]
  fn test_content_item_text() {
    let item = ContentItem::text("hello");
    assert_eq!(item.content_type, "text");
    assert_eq!(item.text, "hello");
  }

  #[test]
  fn test_tool_call_result_success() {
    let result = ToolCallResult::success("ok");
    assert_eq!(result.content.len(), 1);
    assert_eq!(result.content[0].text, "ok");
    assert!(result.is_error.is_none());
  }

  #[test]
  fn test_tool_call_result_error() {
    let result = ToolCallResult::error("failed");
    assert_eq!(result.content.len(), 1);
    assert_eq!(result.is_error, Some(true));
  }

  #[test]
  fn test_tool_call_result_success_json() {
    let value = serde_json::json!({"status": "ok"});
    let result = ToolCallResult::success_json(&value);
    assert!(result.content[0].text.contains("status"));
    assert!(result.is_error.is_none());
  }

  // --- ToolRegistry tests ---

  #[test]
  fn test_tool_registry_register_and_get() {
    let mut registry = ToolRegistry::new();
    assert!(registry.is_empty());
    registry.register(Arc::new(EchoTool));
    assert_eq!(registry.len(), 1);
    assert!(registry.get("echo").is_some());
    assert!(registry.get("nonexistent").is_none());
  }

  #[test]
  fn test_tool_registry_definitions() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(EchoTool));
    let defs = registry.definitions();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].name, "echo");
    assert!(!defs[0].description.is_empty());
  }

  // --- Default tools registration test ---

  #[test]
  fn test_mcp_server_with_default_tools() {
    let server = McpServer::with_default_tools();
    let defs = server.registry().definitions();
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"apmw_add"));
    assert!(names.contains(&"apmw_detect"));
    assert!(names.contains(&"apmw_scan"));
    assert!(names.contains(&"apmw_scan_container"));
    assert!(names.contains(&"apmw_clone"));
    assert!(names.contains(&"apmw_status"));
    assert!(names.contains(&"apmw_list_jobs"));
    assert_eq!(defs.len(), 7);
  }

  // --- Initialize handshake test ---

  #[tokio::test]
  async fn test_mcp_handshake_initialize() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new(
      r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
    );

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 1);
    let resp: JsonRpcResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.id, RequestId::Number(1));
    let protocol_version = resp.result.get("protocol_version").and_then(|v| v.as_str());
    assert_eq!(protocol_version, Some(PROTOCOL_VERSION));
    let server_name = resp
      .result
      .get("server_info")
      .and_then(|s| s.get("name"))
      .and_then(|n| n.as_str());
    assert_eq!(server_name, Some(SERVER_NAME));
  }

  // --- tools/list test ---

  #[tokio::test]
  async fn test_mcp_tools_list() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 1);
    let resp: JsonRpcResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.id, RequestId::Number(2));
    let tools = resp.result.get("tools").and_then(|v| v.as_array());
    assert!(tools.is_some());
    assert_eq!(tools.unwrap().len(), 7);
  }

  // --- tools/call with unknown tool ---

  #[tokio::test]
  async fn test_mcp_tools_call_unknown_tool() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new(
      r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"nonexistent","arguments":{}}}"#,
    );

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 1);
    let resp: JsonRpcErrorResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.id, RequestId::Number(3));
    assert_eq!(resp.error.code, error_codes::INVALID_PARAMS);
  }

  // --- tools/call with missing name ---

  #[tokio::test]
  async fn test_mcp_tools_call_missing_name() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new(
      r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"arguments":{}}}"#,
    );

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    let resp: JsonRpcErrorResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.error.code, error_codes::INVALID_PARAMS);
  }

  // --- Notification handling (no response) ---

  #[tokio::test]
  async fn test_mcp_notification_no_response() {
    let server = McpServer::with_default_tools();
    let mut transport =
      InMemoryTransport::new(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert!(
      output.is_empty(),
      "notifications should not produce a response"
    );
  }

  // --- Parse error handling ---

  #[tokio::test]
  async fn test_mcp_parse_error() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new("not valid json");

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 1);
    let resp: JsonRpcErrorResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.error.code, error_codes::PARSE_ERROR);
  }

  // --- Method not found ---

  #[tokio::test]
  async fn test_mcp_method_not_found() {
    let server = McpServer::with_default_tools();
    let mut transport =
      InMemoryTransport::new(r#"{"jsonrpc":"2.0","id":5,"method":"unknown/method"}"#);

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    let resp: JsonRpcErrorResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp.error.code, error_codes::METHOD_NOT_FOUND);
  }

  // --- apmw_detect tool test ---

  #[tokio::test]
  async fn test_mcp_detect_tool() {
    let tool = DetectTool::new();
    let dir = TempDir::new().unwrap();
    std::fs::write(
      dir.path().join("Cargo.toml"),
      r#"[package]
name = "test"
version = "0.1.0""#,
    )
    .unwrap();

    let args = serde_json::json!({"project_dir": dir.path().to_string_lossy()});
    let result = tool.call(&args).await;
    assert!(result.is_error.is_none());
    assert!(result.content[0].text.contains("cargo"));
  }

  #[tokio::test]
  async fn test_mcp_detect_tool_via_server() {
    let server = McpServer::with_default_tools();
    let dir = TempDir::new().unwrap();
    std::fs::write(
      dir.path().join("Cargo.toml"),
      r#"[package]
name = "test"
version = "0.1.0""#,
    )
    .unwrap();

    let dir_str = dir.path().to_string_lossy().to_string();
    let request = format!(
      r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"apmw_detect","arguments":{{"project_dir":"{}"}}}}}}"#,
      dir_str
    );

    let mut transport = InMemoryTransport::new(&request);
    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 1);
    let resp: JsonRpcResponse = serde_json::from_str(&output[0]).unwrap();
    let content = resp.result.get("content").and_then(|c| c.as_array());
    assert!(content.is_some());
    let text = content.unwrap()[0].get("text").and_then(|t| t.as_str());
    assert!(text.unwrap().contains("cargo"));
  }

  // --- apmw_add tool test (dry-run) ---

  #[tokio::test]
  async fn test_mcp_add_tool_dry_run() {
    let tool = AddTool::new();
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"test"}"#).unwrap();

    let args = serde_json::json!({
      "package": "some-test-pkg-xyz",
      "project_dir": dir.path().to_string_lossy(),
      "dry_run": true,
      "no_scan": true,
      "manager": "pnpm"
    });
    let result = tool.call(&args).await;
    assert!(result.is_error.is_none());
    assert!(result.content[0].text.contains("some-test-pkg-xyz"));
  }

  // --- apmw_scan tool test (no_scan mode) ---

  #[tokio::test]
  async fn test_mcp_scan_tool_no_scan() {
    let tool = ScanTool::new();
    let dir = TempDir::new().unwrap();

    let args = serde_json::json!({
      "packages": [{"name": "test-pkg", "dir": dir.path().to_string_lossy()}],
      "no_scan": true
    });
    let result = tool.call(&args).await;
    assert!(result.is_error.is_none());
    assert!(result.content[0].text.contains("test-pkg"));
  }

  #[tokio::test]
  async fn test_mcp_scan_tool_missing_packages() {
    let tool = ScanTool::new();
    let args = serde_json::json!({});
    let result = tool.call(&args).await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("packages"));
  }

  // --- apmw_scan_container tool test (missing image) ---

  #[tokio::test]
  async fn test_mcp_scan_container_missing_image() {
    let tool = ScanContainerTool::new();
    let args = serde_json::json!({});
    let result = tool.call(&args).await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("image"));
  }

  // --- apmw_clone tool test (missing repo) ---

  #[tokio::test]
  async fn test_mcp_clone_missing_repo() {
    let tool = CloneTool::new();
    let args = serde_json::json!({});
    let result = tool.call(&args).await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("repo"));
  }

  // --- apmw_status tool test ---

  #[tokio::test]
  async fn test_mcp_status_tool() {
    let tool = StatusTool::new();
    let result = tool.call(&serde_json::json!({})).await;
    assert!(result.is_error.is_none());
    assert!(result.content[0].text.contains("version"));
    assert!(result.content[0].text.contains("daemon_running"));
  }

  // --- apmw_list_jobs tool test ---

  #[tokio::test]
  async fn test_mcp_list_jobs_tool() {
    let tool = ListJobsTool::new();
    let result = tool.call(&serde_json::json!({})).await;
    // Daemon is not running in test, so should return empty array.
    assert!(result.is_error.is_none());
    assert!(result.content[0].text.contains("[]"));
  }

  // --- Concurrent tool calls test ---

  #[tokio::test]
  async fn test_mcp_concurrent_tool_calls() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowTool {
      counter: Arc<AtomicUsize>,
    }

    impl Tool for SlowTool {
      fn name(&self) -> &str {
        "slow"
      }
      fn description(&self) -> &str {
        "A slow tool for concurrency testing."
      }
      fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object" })
      }
      fn call(&self, _arguments: &Value) -> ToolFuture<'_> {
        let counter = self.counter.clone();
        Box::pin(async move {
          let count = counter.fetch_add(1, Ordering::SeqCst);
          tokio::time::sleep(std::time::Duration::from_millis(50)).await;
          ToolCallResult::success(format!("call {count}"))
        })
      }
    }

    let counter = Arc::new(AtomicUsize::new(0));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(SlowTool {
      counter: counter.clone(),
    }));
    let server = McpServer::new(registry, 4);

    // Send 4 concurrent tool calls.
    let mut transport = InMemoryTransport::new(
      r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"slow","arguments":{}}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"slow","arguments":{}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"slow","arguments":{}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"slow","arguments":{}}}"#,
    );

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    assert_eq!(output.len(), 4, "should have 4 responses");
    // All 4 calls should have executed.
    assert_eq!(counter.load(Ordering::SeqCst), 4);
  }

  // --- Full protocol sequence test ---

  #[tokio::test]
  async fn test_mcp_full_protocol_sequence() {
    let server = McpServer::with_default_tools();
    let mut transport = InMemoryTransport::new(
      r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"apmw_status","arguments":{}}}"#,
    );

    server.run(&mut transport).await.unwrap();

    let output = transport.output();
    // initialize + tools/list + tools/call = 3 responses (notification has no response)
    assert_eq!(output.len(), 3);

    // First response: initialize
    let resp1: JsonRpcResponse = serde_json::from_str(&output[0]).unwrap();
    assert_eq!(resp1.id, RequestId::Number(1));
    assert!(resp1.result.get("protocol_version").is_some());

    // Second response: tools/list
    let resp2: JsonRpcResponse = serde_json::from_str(&output[1]).unwrap();
    assert_eq!(resp2.id, RequestId::Number(2));
    let tools = resp2.result.get("tools").and_then(|v| v.as_array());
    assert_eq!(tools.unwrap().len(), 7);

    // Third response: tools/call apmw_status
    let resp3: JsonRpcResponse = serde_json::from_str(&output[2]).unwrap();
    assert_eq!(resp3.id, RequestId::Number(3));
    let content = resp3.result.get("content").and_then(|c| c.as_array());
    assert!(content.is_some());
    let text = content.unwrap()[0].get("text").and_then(|t| t.as_str());
    assert!(text.unwrap().contains("version"));
  }

  // --- StdioTransport smoke test (just verify it constructs) ---

  #[test]
  fn test_stdio_transport_constructs() {
    let _ = StdioTransport::new();
  }

  // --- Helper function tests ---

  #[test]
  fn test_get_str_default() {
    let args = serde_json::json!({"a": "b"});
    assert_eq!(get_str(&args, "a", "default"), "b");
    assert_eq!(get_str(&args, "missing", "default"), "default");
  }

  #[test]
  fn test_get_opt_str() {
    let args = serde_json::json!({"a": "b"});
    assert_eq!(get_opt_str(&args, "a"), Some("b".to_string()));
    assert_eq!(get_opt_str(&args, "missing"), None);
  }

  #[test]
  fn test_get_bool() {
    let args = serde_json::json!({"flag": true});
    assert!(get_bool(&args, "flag", false));
    assert!(!get_bool(&args, "missing", false));
    assert!(get_bool(&args, "missing", true));
  }

  #[test]
  fn test_require_str_present() {
    let args = serde_json::json!({"key": "value"});
    assert_eq!(require_str(&args, "key").unwrap(), "value");
  }

  #[test]
  fn test_require_str_missing() {
    let args = serde_json::json!({});
    let result = require_str(&args, "key");
    assert!(result.is_err());
  }

  // --- Tool definitions schema tests ---

  #[test]
  fn test_add_tool_schema() {
    let tool = AddTool::new();
    let schema = tool.input_schema();
    assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
    let props = schema.get("properties").and_then(|v| v.as_object());
    assert!(props.unwrap().contains_key("package"));
  }

  #[test]
  fn test_detect_tool_schema() {
    let tool = DetectTool::new();
    let schema = tool.input_schema();
    let props = schema.get("properties").and_then(|v| v.as_object());
    assert!(props.unwrap().contains_key("project_dir"));
  }

  #[test]
  fn test_scan_tool_schema() {
    let tool = ScanTool::new();
    let schema = tool.input_schema();
    let props = schema.get("properties").and_then(|v| v.as_object());
    assert!(props.unwrap().contains_key("packages"));
  }

  #[test]
  fn test_scan_container_tool_schema() {
    let tool = ScanContainerTool::new();
    let schema = tool.input_schema();
    let props = schema.get("properties").and_then(|v| v.as_object());
    assert!(props.unwrap().contains_key("image"));
  }

  #[test]
  fn test_clone_tool_schema() {
    let tool = CloneTool::new();
    let schema = tool.input_schema();
    let props = schema.get("properties").and_then(|v| v.as_object());
    assert!(props.unwrap().contains_key("repo"));
  }

  // --- Suppress unused import warning for Cursor in test module ---

  #[test]
  fn _ensure_cursor_used() {
    let _ = Cursor::new(Vec::<u8>::new());
  }
}
