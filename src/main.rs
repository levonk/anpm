//! apmw — All Package Manager Wrapper
//!
//! Binary entry point. The CLI is a thin client over a local socket
//! that communicates with the apmw daemon.

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompleteShell};

use apmw::agent::{default_shim_dir, HookManager};
use apmw::audit::{detect_caller_program, detect_terminal_type, AuditLogEntry, AuditLogWriter};
use apmw::cli::{Cli, Commands, GovernanceSubcommand, Shell};
use apmw::clone::{default_clone_dest, CloneEngine};
use apmw::config;
use apmw::daemon::{DaemonManager, JobId};
use apmw::detect::{DetectionEngine, DetectionResult};
use apmw::error::ApmwError;
use apmw::governance::{GovernanceEngine, ReqwestSpecClient, SpecLoader};
use apmw::output::OutputDispatcher;
use apmw::path_scan::PathScanner;

fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();

  if cli.install {
    if cli.intercept {
      return run_install_intercept();
    }
    return run_install(cli.shell);
  }

  if cli.uninstall {
    if cli.intercept {
      return run_uninstall_intercept();
    }
    return run_uninstall();
  }

  // --daemon: run the daemon process directly.
  if cli.global.daemon {
    let rt = tokio::runtime::Runtime::new()?;
    return rt.block_on(async {
      let manager = DaemonManager::new();
      manager.start().await.map_err(|e| anyhow::anyhow!(e))
    });
  }

  // The remaining daemon-related flags (--list-jobs, --cancel-job) and
  // subcommands that may need the daemon are handled in an async runtime.
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async_main(cli))
}

/// Async entry point for daemon-aware dispatch.
async fn async_main(cli: Cli) -> anyhow::Result<()> {
  // --no-daemon: force synchronous in-process operation.
  let no_daemon = cli.global.no_daemon;

  if cli.global.list_jobs {
    if no_daemon {
      // In --no-daemon mode there is no daemon to query; report empty.
      println!("No background jobs (--no-daemon mode).");
      return Ok(());
    }
    let manager = DaemonManager::new();
    if !manager.is_running().await {
      println!("Daemon is not running. No background jobs.");
      return Ok(());
    }
    let jobs = manager.list_jobs().await.map_err(|e| anyhow::anyhow!(e))?;
    if jobs.is_empty() {
      println!("No background jobs running.");
    } else {
      println!(
        "{:<16} {:<10} {:<10} DESCRIPTION",
        "JOB_ID", "STATUS", "KIND"
      );
      for job in &jobs {
        println!(
          "{:<16} {:<10} {:<10} {}",
          job.id.as_str(),
          job.status.to_string(),
          job.kind.0,
          job.description,
        );
      }
    }
    return Ok(());
  }

  if let Some(job_id) = &cli.global.cancel_job {
    if no_daemon {
      return Err(anyhow::anyhow!(ApmwError::Daemon(
        "--cancel-job requires the daemon, but --no-daemon was set".to_string()
      )));
    }
    let manager = DaemonManager::new();
    if !manager.is_running().await {
      return Err(anyhow::anyhow!(ApmwError::Daemon(
        "daemon is not running; cannot cancel job".to_string()
      )));
    }
    let id = JobId::new(job_id.clone());
    let status = manager
      .cancel_job(&id)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;
    println!("Job {job_id} cancelled (status: {status}).");
    return Ok(());
  }

  // For subcommands that require background operations, enforce --no-daemon.
  if no_daemon {
    if let Some(Commands::Clone { .. }) = cli.command {
      return Err(anyhow::anyhow!(ApmwError::Daemon(
        "clone requires the daemon, but --no-daemon was set".to_string()
      )));
    }
  }

  dispatch_subcommand(cli).await
}

/// Dispatch subcommands (non-daemon-related logic).
async fn dispatch_subcommand(cli: Cli) -> anyhow::Result<()> {
  // If a global --manager override is set, record it in the audit log and
  // validate the binary exists on PATH (warning if not found).
  if let Some(ref manager) = cli.global.manager {
    record_manager_override(manager);
    check_manager_binary(manager);
  }

  match cli.command {
    Some(Commands::Install { package, dev }) => {
      let dir = std::env::current_dir()?;
      let on_risk = cli
        .global
        .on_risk
        .map(|a| match a {
          apmw::cli::OnRiskAction::Prompt => apmw::security::OnRiskMode::Prompt,
          apmw::cli::OnRiskAction::Abort => apmw::security::OnRiskMode::Error,
          apmw::cli::OnRiskAction::Proceed => apmw::security::OnRiskMode::Warn,
          apmw::cli::OnRiskAction::Quarantine => apmw::security::OnRiskMode::Skip,
        })
        .unwrap_or_default();

      let engine_config = apmw::install::AddEngineConfig {
        dev,
        dry_run: cli.global.dry_run,
        no_scan: cli.global.no_scan,
        scan_only: cli.global.scan_only,
        manager_override: cli.global.manager.clone(),
        version_spec: String::new(),
        use_devbox: true,
        on_risk,
        min_age: apmw::version::MinAgeDaysConfig::default(),
      };

      let engine = apmw::install::AddEngine::new();
      let result = engine.run(&package, &dir, &engine_config).await?;

      // Build the output.
      let mut dispatcher = OutputDispatcher::from_flags(
        cli.global.human,
        cli.global.json,
        cli.global.fields.as_deref(),
        cli.global.full,
      );
      dispatcher.schema = apmw::output::Schema::with_fields_str(
        vec![
          "package".into(),
          "manager".into(),
          "canonical_manager".into(),
          "status".into(),
        ],
        cli.global.fields.as_deref(),
      );

      let item = serde_json::to_value(&result).unwrap_or(serde_json::json!({}));
      let help = vec![
        format!("apmw install {} --dev", package),
        format!("apmw install {} --dry-run", package),
      ];
      let output = dispatcher.render_item(&item, &help);

      // Print a summary line with "via {manager}" for backward compatibility
      // with existing integration tests, then the structured output.
      let dev_tag = if dev { " (dev)" } else { "" };
      let manager_label = if result.manager.is_empty() {
        result.canonical_manager.clone()
      } else {
        result.manager.clone()
      };
      if manager_label.is_empty() {
        println!("Installed {package}{dev_tag}");
      } else {
        println!("Installed {package}{dev_tag} via {manager_label}");
      }
      println!("{output}");
    }
    Some(Commands::Detect) => {
      let dir = std::env::current_dir()?;

      // When --manager is set, skip auto-detection and use the forced manager.
      let results = if let Some(ref manager) = cli.global.manager {
        tracing::info!(
          manager = manager,
          "Detection skipped due to --manager override"
        );
        vec![DetectionResult::from_forced_manager(manager)]
      } else {
        let engine = DetectionEngine::new();
        engine.detect(&dir)?
      };

      // Write audit log entry.
      let detected_names: Vec<String> = results.iter().map(|r| r.manager.clone()).collect();
      let action = if detected_names.is_empty() {
        "no package manager detected".to_string()
      } else {
        format!("detected: {}", detected_names.join(", "))
      };
      let request = "detect".to_string();
      let terminal_type = detect_terminal_type();
      let caller_program = detect_caller_program();
      let tools_used = detected_names.clone();
      let entry = AuditLogEntry::now(request, action, terminal_type, caller_program, tools_used);
      if let Ok(writer) = AuditLogWriter::new() {
        let _ = writer.append(&entry);
      }

      // Output in TOON (agent mode) or human-readable (human mode).
      let mut dispatcher = OutputDispatcher::from_flags(
        cli.global.human,
        cli.global.json,
        cli.global.fields.as_deref(),
        cli.global.full,
      );
      // Use detection-specific schema fields instead of the default package schema.
      dispatcher.schema = apmw::output::Schema::with_fields_str(
        vec![
          "manager".into(),
          "display_name".into(),
          "ecosystem".into(),
          "confidence".into(),
        ],
        cli.global.fields.as_deref(),
      );

      if results.is_empty() {
        let help = vec![
          "apmw detect --human".to_string(),
          "apmw install <package>".to_string(),
        ];
        println!("{}", dispatcher.render_empty(&help));
      } else {
        let items: Vec<serde_json::Value> = results
          .iter()
          .map(|r| serde_json::to_value(r).unwrap_or(serde_json::json!({})))
          .collect();
        let help = vec![
          format!("apmw install <package> --manager {}", results[0].manager),
          "apmw detect --human".to_string(),
        ];
        println!("{}", dispatcher.render_list(&items, &help));
      }
    }
    Some(Commands::Status) => {
      println!("apmw v{}", apmw::version());
    }
    Some(Commands::Clone { ref package }) => {
      handle_clone(package, &cli).await?;
    }
    Some(Commands::Scan { package }) => match package {
      Some(p) => println!("Scanning {p}..."),
      None => println!("Scanning current project..."),
    },
    Some(Commands::Suggest { package }) => {
      println!("Suggestions for {package}...");
    }
    Some(Commands::Info { package }) => {
      println!("Info for {package}...");
    }
    Some(Commands::AuditLog) => {
      println!("Audit log:");
    }
    Some(Commands::Config { init, show }) => {
      if init {
        let path = config::user_config_path();
        match config::initialize_config_file(&path) {
          Ok(true) => println!("Created config at {}", path.display()),
          Ok(false) => println!("Config already exists at {}", path.display()),
          Err(e) => {
            eprintln!("Failed to initialize config: {e}");
            return Err(anyhow::anyhow!(e));
          }
        }
      }
      if show {
        println!("Resolved configuration:");
        let cfg = config::ApmwConfig::default();
        println!("  daemon_enabled: {}", cfg.daemon_enabled);
        println!("  min_release_age_days: {}", cfg.min_release_age_days);
        println!("  agent_mode: {}", cfg.agent_mode);
      }
      if !init && !show {
        println!("Run 'apmw config --show' to view config or 'apmw config --init' to create it.");
      }
    }
    Some(Commands::Governance { subcommand }) => match subcommand {
      GovernanceSubcommand::Refresh => {
        let loader = SpecLoader::new();
        let client = ReqwestSpecClient::new();
        match loader.refresh(&client) {
          Ok(spec) => {
            println!("Governance spec refreshed (version: {}).", spec.version);
            if spec.rules.is_empty() {
              println!("No governance rules found in spec.");
            } else {
              println!("Loaded {} governance rule(s):", spec.rules.len());
              for rule in &spec.rules {
                println!("  {} {} — {}", rule.governance_type, rule.tool, {
                  rule
                    .message
                    .clone()
                    .unwrap_or_else(|| "(no message)".to_string())
                });
              }
            }
          }
          Err(e) => {
            eprintln!("Failed to refresh governance spec: {e}");
            return Err(anyhow::anyhow!(e));
          }
        }
      }
    },
    Some(Commands::Intercept { tool, args }) => {
      // Invoked by PATH shims. Evaluate governance + security, then print
      // the effective tool and args for the shim to exec.
      let engine = GovernanceEngine::default();
      let mgr = HookManager::new(engine);
      match mgr.intercept(&tool, &args) {
        Ok(result) => {
          if result.decision.run_security_scan {
            tracing::info!(
              tool = tool,
              effective = result.effective_tool.as_str(),
              "intercepted call requires security scan"
            );
          }
          // Output the effective command so the shim can exec it.
          // Format: <tool>\0<arg1>\0<arg2>...
          // The shim reads this and execs the effective tool.
          let mut parts = vec![result.effective_tool.clone()];
          parts.extend(result.effective_args.iter().cloned());
          println!("{}", parts.join("\u{0}"));
        }
        Err(e) => {
          eprintln!("apmw intercept: {e}");
          return Err(anyhow::anyhow!(e));
        }
      }
    }
    None => {
      println!("apmw v{} — run 'apmw --help' for usage", apmw::version());
    }
  }

  Ok(())
}

/// Handle the `apmw clone <repo>` command.
///
/// Performs a historyless clone, writes a local `.gitignore`, and invokes
/// AST indexing. In daemon mode, the clone runs as a background job. In
/// synchronous mode, it runs in-process. Output is in TOON format in agent
/// mode. An audit log entry is written for each clone.
async fn handle_clone(package: &str, cli: &Cli) -> anyhow::Result<()> {
  let dest_dir = default_clone_dest();
  let engine = CloneEngine::new();

  // Write audit log entry for the clone request.
  let terminal_type = detect_terminal_type();
  let caller_program = detect_caller_program();
  let entry = AuditLogEntry::now(
    format!("clone {package}"),
    format!("historyless clone to {}", dest_dir.display()),
    terminal_type,
    &caller_program,
    vec!["git".to_string()],
  );
  if let Ok(writer) = AuditLogWriter::new() {
    let _ = writer.append(&entry);
  }

  // In daemon mode, run the clone as a background job.
  if cli.global.daemon && !cli.global.no_daemon {
    let manager = DaemonManager::new();
    if manager.is_running().await {
      return run_clone_as_job(&manager, &engine, package, &dest_dir, cli).await;
    }
    // Daemon not running — auto-spawn then submit the job.
    if let Err(e) = manager.auto_spawn().await {
      tracing::warn!(error = %e, "Could not start daemon — running clone synchronously");
    } else if manager.is_running().await {
      return run_clone_as_job(&manager, &engine, package, &dest_dir, cli).await;
    }
  }

  // Synchronous mode: run the clone in-process.
  let result = engine
    .clone_repo(package, &dest_dir)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  // Write a completion audit log entry.
  let completion_entry = AuditLogEntry::now(
    format!("clone {package}"),
    format!(
      "cloned to {} (ast_tool={}, indexed={}, files={})",
      result.path, result.ast_tool, result.indexed, result.file_count
    ),
    terminal_type,
    caller_program,
    vec!["git".to_string()],
  );
  if let Ok(writer) = AuditLogWriter::new() {
    let _ = writer.append(&completion_entry);
  }

  // Output in TOON format (agent mode) or human-readable.
  let mut dispatcher = OutputDispatcher::from_flags(
    cli.global.human,
    cli.global.json,
    cli.global.fields.as_deref(),
    cli.global.full,
  );
  dispatcher.schema = apmw::output::Schema::with_fields_str(
    vec![
      "repo".into(),
      "path".into(),
      "ast_tool".into(),
      "indexed".into(),
      "file_count".into(),
    ],
    cli.global.fields.as_deref(),
  );

  let item = serde_json::to_value(&result).unwrap_or(serde_json::json!({}));
  let help = vec![
    format!("apmw clone {}", package),
    format!("apmw --daemon clone {}", package),
  ];
  let output = dispatcher.render_item(&item, &help);
  println!("{output}");

  Ok(())
}

/// Run a clone as a background job via the daemon's JobManager.
///
/// Creates a job, spawns the clone task, and returns immediately with the
/// job ID in TOON format.
async fn run_clone_as_job(
  manager: &DaemonManager,
  engine: &CloneEngine,
  package: &str,
  dest_dir: &std::path::Path,
  cli: &Cli,
) -> anyhow::Result<()> {
  // Create a job via the daemon's IPC.
  let client = apmw::daemon::SocketClient::new(manager.socket_path());
  let resp = client
    .send(&apmw::daemon::Request::CreateJob {
      kind: "clone".to_string(),
      description: format!("clone {package}"),
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
  let job_id = match resp {
    apmw::daemon::Response::JobCreated { id } => id,
    other => {
      return Err(anyhow::anyhow!(ApmwError::Daemon(format!(
        "unexpected response to create-job: {other:?}"
      ))))
    }
  };

  // Spawn the clone task. The engine is cloned (it's cheap and stateless)
  // so the spawned task owns its own copy and does not borrow from the
  // caller's stack frame.
  let package_clone = package.to_string();
  let dest_clone = dest_dir.to_path_buf();
  let socket_path = manager.socket_path().to_path_buf();
  let job_id_clone = job_id.clone();
  let engine_owned = engine.clone();
  tokio::spawn(async move {
    let result = engine_owned.clone_repo(&package_clone, &dest_clone).await;
    let client = apmw::daemon::SocketClient::new(&socket_path);
    match result {
      Ok(r) => {
        tracing::info!(
          job_id = %job_id_clone,
          repo = %r.repo,
          "clone job completed"
        );
        // Best-effort: mark the job as completed via IPC (if supported).
        let _ = client
          .send(&apmw::daemon::Request::GetJob {
            id: job_id_clone.clone(),
          })
          .await;
      }
      Err(e) => {
        tracing::error!(job_id = %job_id_clone, error = %e, "clone job failed");
      }
    }
  });

  // Output the job ID in TOON format.
  let mut dispatcher = OutputDispatcher::from_flags(
    cli.global.human,
    cli.global.json,
    cli.global.fields.as_deref(),
    cli.global.full,
  );
  dispatcher.schema = apmw::output::Schema::with_fields_str(
    vec!["job_id".into(), "status".into(), "repo".into()],
    cli.global.fields.as_deref(),
  );
  let item = serde_json::json!({
    "job_id": job_id.as_str(),
    "status": "pending",
    "repo": package,
  });
  let help = vec![
    format!("apmw --list-jobs"),
    format!("apmw --cancel-job {}", job_id),
  ];
  let output = dispatcher.render_item(&item, &help);
  println!("{output}");

  Ok(())
}

/// Generate shell completions and initialize the config file.
fn run_install(shell: Option<Shell>) -> anyhow::Result<()> {
  let shells = match shell {
    Some(s) => vec![s],
    None => vec![Shell::Bash, Shell::Zsh, Shell::Fish],
  };

  let mut cmd = Cli::command();
  for s in &shells {
    let complete_shell = match s {
      Shell::Bash => CompleteShell::Bash,
      Shell::Zsh => CompleteShell::Zsh,
      Shell::Fish => CompleteShell::Fish,
    };
    let mut buf = Vec::new();
    generate(complete_shell, &mut cmd, "apmw", &mut buf);
    let script = String::from_utf8_lossy(&buf);
    // In a real implementation this would write to the appropriate completion
    // directory. For now we emit to stdout so the user can redirect.
    let _ = std::io::Write::write_all(&mut std::io::stdout(), &buf);
    eprintln!(
      "# Generated {s} completions ({len} bytes)",
      s = s,
      len = script.len()
    );
  }

  let config_path = config::user_config_path();
  match config::initialize_config_file(&config_path) {
    Ok(true) => println!("Created config at {}", config_path.display()),
    Ok(false) => println!("Config already exists at {}", config_path.display()),
    Err(e) => {
      eprintln!("Failed to initialize config: {e}");
      return Err(anyhow::anyhow!(e));
    }
  }

  Ok(())
}

/// Remove generated completions and config (best-effort).
fn run_uninstall() -> anyhow::Result<()> {
  let config_path = config::user_config_path();
  if config_path.exists() {
    match std::fs::remove_file(&config_path) {
      Ok(()) => println!("Removed config at {}", config_path.display()),
      Err(e) => eprintln!("Could not remove config at {}: {e}", config_path.display()),
    }
  } else {
    println!("No config found at {}", config_path.display());
  }

  // Completion files are system-specific; log what would be removed.
  println!(
    "Completion scripts: remove _apmw / apmw.bash / apmw.fish from your shell completion dirs."
  );
  Ok(())
}

/// Install PATH shims that intercept package manager calls (hard intercept).
///
/// Shims are written to the default shim directory. The user should add this
/// directory to the front of their PATH so the shims take precedence over the
/// real binaries.
fn run_install_intercept() -> anyhow::Result<()> {
  let dir = default_shim_dir().map_err(|e| anyhow::anyhow!(e))?;
  let engine = GovernanceEngine::default();
  let mgr = HookManager::new(engine);
  match mgr.install_shims(&dir) {
    Ok(installed) => {
      println!(
        "Installed {} intercept shim(s) to {}",
        installed.len(),
        dir.display()
      );
      println!();
      println!("Add this directory to the FRONT of your PATH:");
      println!("  export PATH=\"{}:$PATH\"", dir.display());
      println!();
      println!("To remove the shims later: apmw --uninstall --intercept");
      Ok(())
    }
    Err(e) => {
      eprintln!("Failed to install intercept shims: {e}");
      Err(anyhow::anyhow!(e))
    }
  }
}

/// Remove PATH shims that intercept package manager calls.
fn run_uninstall_intercept() -> anyhow::Result<()> {
  let dir = default_shim_dir().map_err(|e| anyhow::anyhow!(e))?;
  let engine = GovernanceEngine::default();
  let mgr = HookManager::new(engine);
  match mgr.uninstall_shims(&dir) {
    Ok(removed) => {
      if removed.is_empty() {
        println!("No intercept shims found in {}", dir.display());
      } else {
        println!(
          "Removed {} intercept shim(s) from {}",
          removed.len(),
          dir.display()
        );
      }
      Ok(())
    }
    Err(e) => {
      eprintln!("Failed to remove intercept shims: {e}");
      Err(anyhow::anyhow!(e))
    }
  }
}

/// Record a `--manager` override in the audit log with source `cli-override`.
///
/// The entry uses the `request` field to identify this as a manager override
/// and the `action` field to record the manager name and source.
fn record_manager_override(manager: &str) {
  tracing::info!(
    manager = manager,
    "Manager override active (source: cli-override)"
  );

  let terminal_type = detect_terminal_type();
  let caller_program = detect_caller_program();
  let entry = AuditLogEntry::now(
    "manager-override",
    format!("forced manager: {manager} (source: cli-override)"),
    terminal_type,
    caller_program,
    vec![manager.to_string()],
  );
  if let Ok(writer) = AuditLogWriter::new() {
    let _ = writer.append(&entry);
  }
}

/// Check that the forced manager binary exists on PATH.
///
/// Emits a warning if the binary is not found. This is a warning rather than
/// a hard error because the manager may be available via a wrapper (e.g.
/// devbox, mise) or may be installed in a non-standard location.
fn check_manager_binary(manager: &str) {
  let scanner = PathScanner::new();
  let result = scanner.scan(manager);
  match result {
    apmw::path_scan::ScanResult::Found { path, .. } => {
      tracing::info!(
        manager = manager,
        path = %path.display(),
        "Forced manager binary found on PATH"
      );
    }
    apmw::path_scan::ScanResult::Wrapper { command } => {
      tracing::info!(
        manager = manager,
        wrapper = command,
        "Forced manager available via wrapper"
      );
    }
    apmw::path_scan::ScanResult::NotFound => {
      tracing::warn!(
        manager = manager,
        "Forced manager binary not found on PATH — it may be available via a wrapper \
         or may need to be installed before use"
      );
    }
  }
}
