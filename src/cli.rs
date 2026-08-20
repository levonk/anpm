//! CLI argument definitions for apmw.
//!
//! All command-line interface definitions live here so they are reusable and
//! testable independently of the binary entry point. The [`Cli`] struct and
//! [`Commands`] enum follow ADR-20260607001 for standard arguments, security
//! scanning flags, AXI agent-mode flags, and daemon control flags.

use clap::{Args, Parser, Subcommand};
use tracing::warn;

/// The set of valid package manager names accepted by `--manager`.
///
/// Covers every supported manager across all ecosystems per PRD FR-1.3.
/// New package managers can be added by extending this list.
pub const VALID_MANAGERS: &[&str] = &[
  // Node.js ecosystem
  "pnpm", "npm", "yarn", "bun", // Python ecosystem
  "uv", "pip", "poetry", "pipenv", "pdm", "conda", // Rust ecosystem
  "cargo", // Go ecosystem
  "go",    // Ruby ecosystem
  "gem",   // OS-level / OS-wrapper ecosystem
  "brew", "nix", "devbox", "apt", "dnf", "pacman", "winget", "snap", "flatpak",
  // Container / virtualization ecosystem
  "helm", "docker", "podman", // JVM ecosystem
  "maven", "gradle", "sbt", // .NET ecosystem
  "dotnet",
];

/// Validates a manager name against the set of known package managers.
///
/// Returns the validated name on success, or an error message listing all
/// valid options on failure.
pub fn validate_manager_name(name: &str) -> Result<String, String> {
  if VALID_MANAGERS.contains(&name) {
    Ok(name.to_string())
  } else {
    let valid = VALID_MANAGERS.join(", ");
    Err(format!("invalid manager '{name}'. Valid options: {valid}"))
  }
}

/// Custom value parser for the `--manager` flag.
fn manager_value_parser(s: &str) -> Result<String, String> {
  let result = validate_manager_name(s);
  if result.is_err() {
    warn!(manager = s, "Invalid manager name provided via --manager");
  }
  result
}

/// Color output preference (ADR-20260607001 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorChoice {
  /// Automatically detect whether color is supported.
  #[default]
  Auto,
  /// Always emit color codes.
  Always,
  /// Never emit color codes.
  Never,
}

impl std::str::FromStr for ColorChoice {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "auto" => Ok(ColorChoice::Auto),
      "always" => Ok(ColorChoice::Always),
      "never" => Ok(ColorChoice::Never),
      _ => Err(format!(
        "invalid color choice '{s}', expected auto|always|never"
      )),
    }
  }
}

impl std::fmt::Display for ColorChoice {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ColorChoice::Auto => write!(f, "auto"),
      ColorChoice::Always => write!(f, "always"),
      ColorChoice::Never => write!(f, "never"),
    }
  }
}

/// Action to take when a security scan detects risk (ADR-20260607001 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnRiskAction {
  /// Prompt the user for confirmation (default in interactive mode).
  #[default]
  Prompt,
  /// Abort the operation.
  Abort,
  /// Proceed despite the risk.
  Proceed,
  /// Quarantine the package.
  Quarantine,
}

impl std::str::FromStr for OnRiskAction {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "prompt" => Ok(OnRiskAction::Prompt),
      "abort" => Ok(OnRiskAction::Abort),
      "proceed" => Ok(OnRiskAction::Proceed),
      "quarantine" => Ok(OnRiskAction::Quarantine),
      _ => Err(format!(
        "invalid on-risk action '{s}', expected prompt|abort|proceed|quarantine"
      )),
    }
  }
}

impl std::fmt::Display for OnRiskAction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OnRiskAction::Prompt => write!(f, "prompt"),
      OnRiskAction::Abort => write!(f, "abort"),
      OnRiskAction::Proceed => write!(f, "proceed"),
      OnRiskAction::Quarantine => write!(f, "quarantine"),
    }
  }
}

/// Shell for which completions are generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
  /// Bash shell.
  Bash,
  /// Zsh shell.
  Zsh,
  /// Fish shell.
  Fish,
}

impl std::str::FromStr for Shell {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_ascii_lowercase().as_str() {
      "bash" => Ok(Shell::Bash),
      "zsh" => Ok(Shell::Zsh),
      "fish" => Ok(Shell::Fish),
      _ => Err(format!("invalid shell '{s}', expected bash|zsh|fish")),
    }
  }
}

impl std::fmt::Display for Shell {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Shell::Bash => write!(f, "bash"),
      Shell::Zsh => write!(f, "zsh"),
      Shell::Fish => write!(f, "fish"),
    }
  }
}

/// Global flags shared across all subcommands (ADR-20260607001 §1-12, §36-45).
///
/// These are flattened into the top-level [`Cli`] struct so they are available
/// alongside any subcommand.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct GlobalArgs {
  // --- I/O and output flags (ADR §5, §36-45) ---
  /// Emit output as JSON (machine-readable).
  #[arg(long, global = true)]
  pub json: bool,

  /// Color output: auto, always, never.
  #[arg(long, value_name = "WHEN", global = true, default_value = "auto")]
  pub color: ColorChoice,

  /// Human-readable output (escape hatch for AXI/TOON mode).
  #[arg(long, global = true)]
  pub human: bool,

  // --- Logging flags (ADR §6) ---
  /// Increase verbosity (can be repeated: -v, -vv).
  #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
  pub verbose: u8,

  /// Suppress non-error output.
  #[arg(short = 'q', long, global = true)]
  pub quiet: bool,

  /// Enable debug-level diagnostics.
  #[arg(long, global = true)]
  pub debug: bool,

  // --- Execution control flags (ADR §8-10) ---
  /// Show what would happen without making changes.
  #[arg(long, global = true)]
  pub dry_run: bool,

  /// Force the operation, bypassing confirmations.
  #[arg(long, global = true)]
  pub force: bool,

  /// Enable interactive/TUI mode.
  #[arg(long = "interactive", visible_alias = "tui", global = true)]
  pub interactive: bool,

  /// Disable pager output.
  #[arg(long, global = true)]
  pub no_pager: bool,

  // --- Security scanning flags (ADR §7) ---
  /// Skip security scanning.
  #[arg(long, global = true)]
  pub no_scan: bool,

  /// Only run the security scan, then exit.
  #[arg(long, global = true)]
  pub scan_only: bool,

  /// Action to take when a security scan detects risk.
  #[arg(long, value_name = "ACTION", global = true)]
  pub on_risk: Option<OnRiskAction>,

  /// Update the security vulnerability database before scanning.
  #[arg(long, global = true)]
  pub update_security_db: bool,

  // --- AXI agent-mode flags (ADR §36-45) ---
  /// Comma-separated list of fields to include in AXI minimal output.
  #[arg(long, value_name = "LIST", global = true)]
  pub fields: Option<String>,

  /// Show full content (escape hatch for AXI truncation).
  #[arg(long, global = true)]
  pub full: bool,

  // --- Daemon flags (ADR §13) ---
  /// Run in daemon mode (long-running background process).
  #[arg(long, global = true)]
  pub daemon: bool,

  /// Disable daemon mode (force synchronous operation).
  #[arg(long, global = true)]
  pub no_daemon: bool,

  /// List background jobs.
  #[arg(long, global = true)]
  pub list_jobs: bool,

  /// Cancel a background job by ID.
  #[arg(long, value_name = "ID", global = true)]
  pub cancel_job: Option<String>,

  // --- Manager override (PRD FR-1.3) ---
  /// Override auto-detection and force a specific package manager.
  ///
  /// When set, detection is skipped entirely and the specified manager is
  /// used. Valid values cover all supported package managers across
  /// ecosystems. See `--help` for the full list.
  #[arg(
    long,
    visible_alias = "use",
    value_name = "NAME",
    global = true,
    value_parser = manager_value_parser,
  )]
  pub manager: Option<String>,

  // --- Telemetry flags (PRD FR-6) ---
  /// Disable telemetry collection for this invocation.
  #[arg(long, global = true)]
  pub no_telemetry: bool,

  /// Print the telemetry payload that would be sent without actually sending it.
  #[arg(long, global = true)]
  pub telemetry_preview: bool,
}

/// The top-level apmw CLI (ADR-20260607001).
#[derive(Parser, Debug, Clone, PartialEq, Eq)]
#[command(name = "apmw")]
#[command(version)]
#[command(about = "All Package Manager Wrapper")]
#[command(
  long_about = "Abstracts every package installer into one intelligent surface.\nDetects the correct package manager, installs tools with install-on-use\nsemantics, and runs security scanning before install."
)]
pub struct Cli {
  /// Generate shell completions and initialize the config file.
  #[arg(long)]
  pub install: bool,

  /// Remove generated completions and config (best-effort cleanup).
  #[arg(long)]
  pub uninstall: bool,

  /// Install or remove PATH shims that intercept package manager calls
  /// (used with `--install` or `--uninstall`).
  #[arg(long)]
  pub intercept: bool,

  /// Shell to generate completions for (used with --install).
  #[arg(long, value_name = "SHELL", requires = "install")]
  pub shell: Option<Shell>,

  /// Print the man page to stdout (groff/troff format) and exit.
  ///
  /// The output can be piped to `man -l -` or saved to a file under
  /// `man/man1/apmw.1`. Equivalent to `man apmw` once installed.
  #[arg(long, exclusive = true)]
  pub man: bool,

  /// Print a brief usage summary and exit.
  ///
  /// Shows a one-line synopsis and the most common commands, suitable for
  /// quick reference. Use `--help` for the full description.
  #[arg(long, exclusive = true)]
  pub usage: bool,

  /// Global flags shared across all subcommands.
  #[command(flatten)]
  pub global: GlobalArgs,

  /// Subcommand to run.
  #[command(subcommand)]
  pub command: Option<Commands>,
}

/// All apmw subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
  /// Install a package or tool.
  Install {
    /// Package name or specifier to install.
    package: String,

    /// Install as a development/build-time dependency.
    #[arg(long)]
    dev: bool,
  },

  /// Detect the package manager for the current project.
  Detect,

  /// Show apmw status and audit log.
  Status,

  /// Create a historyless clone of a repository with AST indexing.
  Clone {
    /// Repository URL or package name to clone.
    package: String,
  },

  /// Scan a package (or the current project) for security issues.
  Scan {
    /// Package name to scan. If omitted, scans the current project.
    package: Option<String>,
  },

  /// Suggest within-ecosystem alternatives for a package.
  Suggest {
    /// Package name to find alternatives for.
    package: String,
  },

  /// Show detailed info about a package.
  Info {
    /// Package name to inspect.
    package: String,
  },

  /// Show the audit log of past operations.
  AuditLog,

  /// View or initialize apmw configuration.
  Config {
    /// Initialize the config file with defaults.
    #[arg(long)]
    init: bool,

    /// Show the resolved configuration.
    #[arg(long)]
    show: bool,
  },

  /// Manage governance rules (refresh the spec, show current rules).
  Governance {
    #[command(subcommand)]
    subcommand: GovernanceSubcommand,
  },

  /// Intercept a package manager call (invoked by PATH shims).
  ///
  /// This subcommand is normally called by the shims installed via
  /// `--install --intercept`. It evaluates governance rules, runs security
  /// scanning, and then delegates to the real binary (or the canonical
  /// alternative if governance forces it).
  Intercept {
    /// The package manager tool that was intercepted (e.g. `pip`, `npm`).
    tool: String,

    /// Arguments to pass through to the (canonical) package manager.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
  },

  /// Start the MCP (Model Context Protocol) server over stdio.
  ///
  /// AI agents (Claude Code, Codex, OpenCode) connect to this server to
  /// invoke apmw operations (add, detect, scan, clone, etc.) as MCP tools.
  /// The server reads line-delimited JSON-RPC 2.0 messages from stdin and
  /// writes responses to stdout.
  Mcp,
}

/// Subcommands for `apmw governance`.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum GovernanceSubcommand {
  /// Force-refresh the cached governance spec from levonk-packages.
  Refresh,
}

#[cfg(test)]
mod tests {
  use super::*;
  use clap::Parser;
  use std::str::FromStr;

  #[test]
  fn test_parse_no_args() {
    let cli = Cli::try_parse_from(["apmw"]).unwrap();
    assert!(cli.command.is_none());
    assert!(!cli.install);
    assert!(!cli.uninstall);
  }

  #[test]
  fn test_parse_install_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "install", "express"]).unwrap();
    match cli.command {
      Some(Commands::Install { package, dev }) => {
        assert_eq!(package, "express");
        assert!(!dev);
      }
      _ => panic!("expected Install command"),
    }
  }

  #[test]
  fn test_parse_install_with_dev_flag() {
    let cli = Cli::try_parse_from(["apmw", "install", "jest", "--dev"]).unwrap();
    match cli.command {
      Some(Commands::Install { package, dev }) => {
        assert_eq!(package, "jest");
        assert!(dev);
      }
      _ => panic!("expected Install command"),
    }
  }

  #[test]
  fn test_parse_install_with_global_manager_override() {
    let cli = Cli::try_parse_from(["apmw", "install", "lodash", "--manager", "pnpm"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("pnpm"));
  }

  #[test]
  fn test_parse_detect_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "detect"]).unwrap();
    assert_eq!(cli.command, Some(Commands::Detect));
  }

  #[test]
  fn test_parse_status_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert_eq!(cli.command, Some(Commands::Status));
  }

  #[test]
  fn test_parse_clone_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "clone", "https://github.com/owner/repo"]).unwrap();
    match cli.command {
      Some(Commands::Clone { package }) => {
        assert_eq!(package, "https://github.com/owner/repo");
      }
      _ => panic!("expected Clone command"),
    }
  }

  #[test]
  fn test_parse_scan_no_package() {
    let cli = Cli::try_parse_from(["apmw", "scan"]).unwrap();
    match cli.command {
      Some(Commands::Scan { package }) => assert!(package.is_none()),
      _ => panic!("expected Scan command"),
    }
  }

  #[test]
  fn test_parse_scan_with_package() {
    let cli = Cli::try_parse_from(["apmw", "scan", "left-pad"]).unwrap();
    match cli.command {
      Some(Commands::Scan { package }) => assert_eq!(package.as_deref(), Some("left-pad")),
      _ => panic!("expected Scan command"),
    }
  }

  #[test]
  fn test_parse_suggest_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "suggest", "npm"]).unwrap();
    match cli.command {
      Some(Commands::Suggest { package }) => assert_eq!(package, "npm"),
      _ => panic!("expected Suggest command"),
    }
  }

  #[test]
  fn test_parse_info_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "info", "react"]).unwrap();
    match cli.command {
      Some(Commands::Info { package }) => assert_eq!(package, "react"),
      _ => panic!("expected Info command"),
    }
  }

  #[test]
  fn test_parse_audit_log_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "audit-log"]).unwrap();
    assert_eq!(cli.command, Some(Commands::AuditLog));
  }

  #[test]
  fn test_parse_config_init() {
    let cli = Cli::try_parse_from(["apmw", "config", "--init"]).unwrap();
    match cli.command {
      Some(Commands::Config { init, show }) => {
        assert!(init);
        assert!(!show);
      }
      _ => panic!("expected Config command"),
    }
  }

  #[test]
  fn test_parse_config_show() {
    let cli = Cli::try_parse_from(["apmw", "config", "--show"]).unwrap();
    match cli.command {
      Some(Commands::Config { init, show }) => {
        assert!(!init);
        assert!(show);
      }
      _ => panic!("expected Config command"),
    }
  }

  #[test]
  fn test_parse_global_json_flag() {
    let cli = Cli::try_parse_from(["apmw", "--json", "status"]).unwrap();
    assert!(cli.global.json);
  }

  #[test]
  fn test_parse_global_color_flag() {
    let cli = Cli::try_parse_from(["apmw", "--color", "never", "status"]).unwrap();
    assert_eq!(cli.global.color, ColorChoice::Never);
  }

  #[test]
  fn test_parse_global_color_default() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert_eq!(cli.global.color, ColorChoice::Auto);
  }

  #[test]
  fn test_parse_global_verbose_flag() {
    let cli = Cli::try_parse_from(["apmw", "-v", "status"]).unwrap();
    assert_eq!(cli.global.verbose, 1);
  }

  #[test]
  fn test_parse_global_verbose_repeated() {
    let cli = Cli::try_parse_from(["apmw", "-vv", "status"]).unwrap();
    assert_eq!(cli.global.verbose, 2);
  }

  #[test]
  fn test_parse_global_quiet_flag() {
    let cli = Cli::try_parse_from(["apmw", "--quiet", "status"]).unwrap();
    assert!(cli.global.quiet);
  }

  #[test]
  fn test_parse_global_debug_flag() {
    let cli = Cli::try_parse_from(["apmw", "--debug", "status"]).unwrap();
    assert!(cli.global.debug);
  }

  #[test]
  fn test_parse_global_dry_run_flag() {
    let cli = Cli::try_parse_from(["apmw", "--dry-run", "install", "pkg"]).unwrap();
    assert!(cli.global.dry_run);
  }

  #[test]
  fn test_parse_global_force_flag() {
    let cli = Cli::try_parse_from(["apmw", "--force", "install", "pkg"]).unwrap();
    assert!(cli.global.force);
  }

  #[test]
  fn test_parse_global_human_flag() {
    let cli = Cli::try_parse_from(["apmw", "--human", "status"]).unwrap();
    assert!(cli.global.human);
  }

  #[test]
  fn test_parse_global_interactive_flag() {
    let cli = Cli::try_parse_from(["apmw", "--interactive", "status"]).unwrap();
    assert!(cli.global.interactive);
  }

  #[test]
  fn test_parse_global_tui_alias() {
    let cli = Cli::try_parse_from(["apmw", "--tui", "status"]).unwrap();
    assert!(cli.global.interactive);
  }

  #[test]
  fn test_parse_global_no_pager_flag() {
    let cli = Cli::try_parse_from(["apmw", "--no-pager", "status"]).unwrap();
    assert!(cli.global.no_pager);
  }

  #[test]
  fn test_parse_security_no_scan_flag() {
    let cli = Cli::try_parse_from(["apmw", "--no-scan", "install", "pkg"]).unwrap();
    assert!(cli.global.no_scan);
  }

  #[test]
  fn test_parse_security_scan_only_flag() {
    let cli = Cli::try_parse_from(["apmw", "--scan-only", "scan"]).unwrap();
    assert!(cli.global.scan_only);
  }

  #[test]
  fn test_parse_security_on_risk_flag() {
    let cli = Cli::try_parse_from(["apmw", "--on-risk", "abort", "install", "pkg"]).unwrap();
    assert_eq!(cli.global.on_risk, Some(OnRiskAction::Abort));
  }

  #[test]
  fn test_parse_security_update_db_flag() {
    let cli = Cli::try_parse_from(["apmw", "--update-security-db", "scan"]).unwrap();
    assert!(cli.global.update_security_db);
  }

  #[test]
  fn test_parse_axi_fields_flag() {
    let cli = Cli::try_parse_from(["apmw", "--fields", "name,version", "info", "pkg"]).unwrap();
    assert_eq!(cli.global.fields.as_deref(), Some("name,version"));
  }

  #[test]
  fn test_parse_axi_full_flag() {
    let cli = Cli::try_parse_from(["apmw", "--full", "info", "pkg"]).unwrap();
    assert!(cli.global.full);
  }

  #[test]
  fn test_parse_daemon_flag() {
    let cli = Cli::try_parse_from(["apmw", "--daemon", "status"]).unwrap();
    assert!(cli.global.daemon);
  }

  #[test]
  fn test_parse_no_daemon_flag() {
    let cli = Cli::try_parse_from(["apmw", "--no-daemon", "status"]).unwrap();
    assert!(cli.global.no_daemon);
  }

  #[test]
  fn test_parse_list_jobs_flag() {
    let cli = Cli::try_parse_from(["apmw", "--list-jobs"]).unwrap();
    assert!(cli.global.list_jobs);
  }

  #[test]
  fn test_parse_cancel_job_flag() {
    let cli = Cli::try_parse_from(["apmw", "--cancel-job", "job-123"]).unwrap();
    assert_eq!(cli.global.cancel_job.as_deref(), Some("job-123"));
  }

  #[test]
  fn test_parse_install_flag() {
    let cli = Cli::try_parse_from(["apmw", "--install"]).unwrap();
    assert!(cli.install);
  }

  #[test]
  fn test_parse_man_flag() {
    let cli = Cli::try_parse_from(["apmw", "--man"]).unwrap();
    assert!(cli.man);
  }

  #[test]
  fn test_parse_usage_flag() {
    let cli = Cli::try_parse_from(["apmw", "--usage"]).unwrap();
    assert!(cli.usage);
  }

  #[test]
  fn test_parse_man_flag_default_false() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert!(!cli.man);
  }

  #[test]
  fn test_parse_usage_flag_default_false() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert!(!cli.usage);
  }

  #[test]
  fn test_parse_install_with_shell() {
    let cli = Cli::try_parse_from(["apmw", "--install", "--shell", "bash"]).unwrap();
    assert!(cli.install);
    assert_eq!(cli.shell, Some(Shell::Bash));
  }

  #[test]
  fn test_parse_uninstall_flag() {
    let cli = Cli::try_parse_from(["apmw", "--uninstall"]).unwrap();
    assert!(cli.uninstall);
  }

  #[test]
  fn test_parse_invalid_color_errors() {
    let result = Cli::try_parse_from(["apmw", "--color", "rainbow", "status"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_invalid_on_risk_errors() {
    let result = Cli::try_parse_from(["apmw", "--on-risk", "delete", "install", "pkg"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_invalid_shell_errors() {
    let result = Cli::try_parse_from(["apmw", "--install", "--shell", "powershell"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_unknown_subcommand_errors() {
    let result = Cli::try_parse_from(["apmw", "frobnicate"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_install_missing_package_errors() {
    let result = Cli::try_parse_from(["apmw", "install"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_clone_missing_package_errors() {
    let result = Cli::try_parse_from(["apmw", "clone"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_suggest_missing_package_errors() {
    let result = Cli::try_parse_from(["apmw", "suggest"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_info_missing_package_errors() {
    let result = Cli::try_parse_from(["apmw", "info"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_color_choice_from_str() {
    assert_eq!(ColorChoice::from_str("auto").unwrap(), ColorChoice::Auto);
    assert_eq!(
      ColorChoice::from_str("ALWAYS").unwrap(),
      ColorChoice::Always
    );
    assert_eq!(ColorChoice::from_str("Never").unwrap(), ColorChoice::Never);
    assert!(ColorChoice::from_str("bad").is_err());
  }

  #[test]
  fn test_color_choice_display() {
    assert_eq!(ColorChoice::Auto.to_string(), "auto");
    assert_eq!(ColorChoice::Always.to_string(), "always");
    assert_eq!(ColorChoice::Never.to_string(), "never");
  }

  #[test]
  fn test_on_risk_action_from_str() {
    assert_eq!(
      OnRiskAction::from_str("prompt").unwrap(),
      OnRiskAction::Prompt
    );
    assert_eq!(
      OnRiskAction::from_str("ABORT").unwrap(),
      OnRiskAction::Abort
    );
    assert_eq!(
      OnRiskAction::from_str("proceed").unwrap(),
      OnRiskAction::Proceed
    );
    assert_eq!(
      OnRiskAction::from_str("Quarantine").unwrap(),
      OnRiskAction::Quarantine
    );
    assert!(OnRiskAction::from_str("bad").is_err());
  }

  #[test]
  fn test_on_risk_action_display() {
    assert_eq!(OnRiskAction::Prompt.to_string(), "prompt");
    assert_eq!(OnRiskAction::Abort.to_string(), "abort");
    assert_eq!(OnRiskAction::Proceed.to_string(), "proceed");
    assert_eq!(OnRiskAction::Quarantine.to_string(), "quarantine");
  }

  #[test]
  fn test_shell_from_str() {
    assert_eq!(Shell::from_str("bash").unwrap(), Shell::Bash);
    assert_eq!(Shell::from_str("ZSH").unwrap(), Shell::Zsh);
    assert_eq!(Shell::from_str("fish").unwrap(), Shell::Fish);
    assert!(Shell::from_str("tcsh").is_err());
  }

  #[test]
  fn test_shell_display() {
    assert_eq!(Shell::Bash.to_string(), "bash");
    assert_eq!(Shell::Zsh.to_string(), "zsh");
    assert_eq!(Shell::Fish.to_string(), "fish");
  }

  // --- Manager override tests (story 02-004) ---

  #[test]
  fn test_validate_manager_name_valid_pnpm() {
    assert_eq!(validate_manager_name("pnpm").unwrap(), "pnpm");
  }

  #[test]
  fn test_validate_manager_name_valid_uv() {
    assert_eq!(validate_manager_name("uv").unwrap(), "uv");
  }

  #[test]
  fn test_validate_manager_name_valid_cargo() {
    assert_eq!(validate_manager_name("cargo").unwrap(), "cargo");
  }

  #[test]
  fn test_validate_manager_name_valid_docker() {
    assert_eq!(validate_manager_name("docker").unwrap(), "docker");
  }

  #[test]
  fn test_validate_manager_name_valid_all() {
    for &name in VALID_MANAGERS {
      assert!(
        validate_manager_name(name).is_ok(),
        "'{name}' should be a valid manager name"
      );
    }
  }

  #[test]
  fn test_validate_manager_name_invalid() {
    let result = validate_manager_name("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("invalid manager 'nonexistent'"));
    assert!(err.contains("pnpm"));
    assert!(err.contains("cargo"));
    assert!(err.contains("docker"));
  }

  #[test]
  fn test_validate_manager_name_invalid_lists_valid_options() {
    let result = validate_manager_name("foobar");
    assert!(result.is_err());
    let err = result.unwrap_err();
    // The error message should list all valid options.
    for &name in VALID_MANAGERS {
      assert!(err.contains(name), "Error message should contain '{name}'");
    }
  }

  #[test]
  fn test_parse_global_manager_flag() {
    let cli = Cli::try_parse_from(["apmw", "--manager", "pnpm", "status"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("pnpm"));
  }

  #[test]
  fn test_parse_global_manager_use_alias() {
    let cli = Cli::try_parse_from(["apmw", "--use", "uv", "status"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("uv"));
  }

  #[test]
  fn test_parse_global_manager_on_detect() {
    let cli = Cli::try_parse_from(["apmw", "--manager", "cargo", "detect"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("cargo"));
    assert_eq!(cli.command, Some(Commands::Detect));
  }

  #[test]
  fn test_parse_global_manager_on_install() {
    let cli = Cli::try_parse_from(["apmw", "install", "express", "--manager", "pnpm"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("pnpm"));
  }

  #[test]
  fn test_parse_global_manager_use_alias_on_install() {
    let cli = Cli::try_parse_from(["apmw", "install", "express", "--use", "npm"]).unwrap();
    assert_eq!(cli.global.manager.as_deref(), Some("npm"));
  }

  #[test]
  fn test_parse_global_manager_not_set() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert!(cli.global.manager.is_none());
  }

  #[test]
  fn test_parse_invalid_manager_errors() {
    let result = Cli::try_parse_from(["apmw", "--manager", "nonexistent", "status"]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("invalid manager"));
    assert!(err.contains("pnpm"));
  }

  #[test]
  fn test_parse_invalid_manager_use_alias_errors() {
    let result = Cli::try_parse_from(["apmw", "--use", "badmgr", "status"]);
    assert!(result.is_err());
  }

  #[test]
  fn test_valid_managers_count() {
    // The story defines 29 valid managers; ensure we have exactly that many.
    assert_eq!(
      VALID_MANAGERS.len(),
      29,
      "Expected 29 valid managers, got {}",
      VALID_MANAGERS.len()
    );
  }

  // --- Telemetry flag tests (story 04-005) ---

  #[test]
  fn test_parse_no_telemetry_flag() {
    let cli = Cli::try_parse_from(["apmw", "--no-telemetry", "status"]).unwrap();
    assert!(cli.global.no_telemetry);
  }

  #[test]
  fn test_parse_no_telemetry_flag_default_false() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert!(!cli.global.no_telemetry);
  }

  #[test]
  fn test_parse_no_telemetry_with_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "install", "express", "--no-telemetry"]).unwrap();
    assert!(cli.global.no_telemetry);
  }

  #[test]
  fn test_parse_telemetry_preview_flag() {
    let cli = Cli::try_parse_from(["apmw", "--telemetry-preview", "status"]).unwrap();
    assert!(cli.global.telemetry_preview);
  }

  #[test]
  fn test_parse_telemetry_preview_default_false() {
    let cli = Cli::try_parse_from(["apmw", "status"]).unwrap();
    assert!(!cli.global.telemetry_preview);
  }

  #[test]
  fn test_parse_telemetry_preview_with_subcommand() {
    let cli = Cli::try_parse_from(["apmw", "detect", "--telemetry-preview"]).unwrap();
    assert!(cli.global.telemetry_preview);
  }

  #[test]
  fn test_parse_both_telemetry_flags() {
    let cli =
      Cli::try_parse_from(["apmw", "--no-telemetry", "--telemetry-preview", "status"]).unwrap();
    assert!(cli.global.no_telemetry);
    assert!(cli.global.telemetry_preview);
  }
}
