//! apmw — All Package Manager Wrapper
//!
//! Binary entry point. The CLI is a thin client over a local socket
//! that communicates with the apmw daemon.

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompleteShell};

use apmw::cli::{Cli, Commands, Shell};
use apmw::config;
use apmw::daemon::{DaemonManager, JobId};
use apmw::error::ApmwError;

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
  match cli.command {
    Some(Commands::Install {
      package,
      dev,
      manager,
    }) => {
      let dev_tag = if dev { " (dev)" } else { "" };
      match &manager {
        Some(m) => println!("Installing {package}{dev_tag} via {m}..."),
        None => println!("Installing {package}{dev_tag}..."),
      }
    }
    Some(Commands::Detect) => {
      println!("Detecting package manager...");
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
