//! apmw — All Package Manager Wrapper
//!
//! Binary entry point. The CLI is a thin client over a local socket
//! that communicates with the apmw daemon.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apmw")]
#[command(version)]
#[command(about = "All Package Manager Wrapper")]
#[command(long_about = "Abstracts every package installer into one intelligent surface.\nDetects the correct package manager, installs tools with install-on-use\nsemantics, and runs security scanning before install.")]
struct Cli {
    /// Run in daemon mode (long-running background process)
    #[arg(long)]
    daemon: bool,

    /// Disable daemon mode (force synchronous operation)
    #[arg(long)]
    no_daemon: bool,

    /// List background jobs
    #[arg(long)]
    list_jobs: bool,

    /// Cancel a background job by ID
    #[arg(long, value_name = "ID")]
    cancel_job: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package or tool
    Install { package: String },
    /// Detect the package manager for the current project
    Detect,
    /// Show apmw status and audit log
    Status,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.list_jobs {
        println!("No background jobs running.");
        return Ok(());
    }

    if let Some(job_id) = cli.cancel_job {
        println!("Cancelling job: {job_id}");
        return Ok(());
    }

    match cli.command {
        Some(Commands::Install { package }) => {
            println!("Installing: {package}");
        }
        Some(Commands::Detect) => {
            println!("Detecting package manager...");
        }
        Some(Commands::Status) => {
            println!("apmw v{}", apmw::version());
        }
        None => {
            println!("apmw v{} — run 'apmw --help' for usage", apmw::version());
        }
    }

    Ok(())
}
