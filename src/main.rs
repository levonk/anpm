//! apmw — All Package Manager Wrapper
//!
//! Binary entry point. The CLI is a thin client over a local socket
//! that communicates with the apmw daemon.

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompleteShell};

use apmw::cli::{Cli, Commands, Shell};
use apmw::config;

fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();

  if cli.install {
    return run_install(cli.shell);
  }

  if cli.uninstall {
    return run_uninstall();
  }

  if cli.global.list_jobs {
    println!("No background jobs running.");
    return Ok(());
  }

  if let Some(job_id) = &cli.global.cancel_job {
    println!("Cancelling job: {job_id}");
    return Ok(());
  }

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
