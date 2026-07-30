//! apmw — All Package Manager Wrapper
//!
//! Binary entry point. The CLI is a thin client over a local socket
//! that communicates with the apmw daemon.

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompleteShell};

use apmw::audit::{detect_caller_program, detect_terminal_type, AuditLogEntry, AuditLogWriter};
use apmw::cli::{Cli, Commands, GovernanceSubcommand, Shell};
use apmw::config;
use apmw::daemon::{DaemonManager, JobId};
use apmw::detect::{DetectionEngine, DetectionResult};
use apmw::error::ApmwError;
use apmw::governance::{ReqwestSpecClient, SpecLoader};
use apmw::output::OutputDispatcher;
use apmw::path_scan::PathScanner;

fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();

  if cli.install {
    return run_install(cli.shell);
  }

  if cli.uninstall {
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
      let dev_tag = if dev { " (dev)" } else { "" };
      match &cli.global.manager {
        Some(m) => println!("Installing {package}{dev_tag} via {m}..."),
        None => println!("Installing {package}{dev_tag}..."),
      }
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
    Some(Commands::Clone { package }) => {
      println!("Cloning {package}...");
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
    None => {
      println!("apmw v{} — run 'apmw --help' for usage", apmw::version());
    }
  }

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
