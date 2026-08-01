//! Asset generator for apmw.
//!
//! Generates man pages (via `clap_mangen`) and shell completion scripts
//! (via `clap_complete`) into the `man/` and `completions/` directories at
//! the repository root.
//!
//! Usage:
//!   cargo run --example gen-assets -- man         # generate man pages
//!   cargo run --example gen-assets -- completions # generate completion scripts
//!   cargo run --example gen-assets -- all         # generate both
//!
//! This is invoked by the `just man` and `just completions` targets.

use std::fs;
use std::io::Write;
use std::path::Path;

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompleteShell};
use clap_mangen::Man;

#[derive(Parser, Debug)]
#[command(
  name = "gen-assets",
  about = "Generate apmw man pages and shell completions"
)]
struct GenArgs {
  /// What to generate: "man", "completions", or "all".
  asset: String,

  /// Output directory for man pages (default: man).
  #[arg(long, default_value = "man")]
  man_dir: String,

  /// Output directory for completions (default: completions).
  #[arg(long, default_value = "completions")]
  completions_dir: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = GenArgs::parse();
  let which = args.asset.as_str();
  let do_man = which == "man" || which == "all";
  let do_completions = which == "completions" || which == "all";

  if !do_man && !do_completions {
    return Err(format!("unknown asset '{which}' (expected man|completions|all)").into());
  }

  let cmd = apmw::cli::Cli::command();

  if do_man {
    let man_dir = Path::new(&args.man_dir);
    fs::create_dir_all(man_dir.join("man1"))?;
    let man = Man::new(cmd.clone());
    let out_path = man_dir.join("man1").join("apmw.1");
    let mut file = fs::File::create(&out_path)?;
    man.render(&mut file)?;
    // Ensure the file ends with a newline.
    file.write_all(b"\n")?;
    println!("Generated man page: {}", out_path.display());
  }

  if do_completions {
    let comp_dir = Path::new(&args.completions_dir);
    fs::create_dir_all(comp_dir)?;
    for shell in [CompleteShell::Bash, CompleteShell::Zsh, CompleteShell::Fish] {
      let mut buf = Vec::new();
      let mut cmd_clone = cmd.clone();
      generate(shell, &mut cmd_clone, "apmw", &mut buf);
      let ext = match shell {
        CompleteShell::Bash => "bash",
        CompleteShell::Zsh => "zsh",
        CompleteShell::Fish => "fish",
        _ => "txt",
      };
      let out_path = comp_dir.join(format!("apmw.{ext}"));
      fs::write(&out_path, &buf)?;
      println!("Generated completion: {}", out_path.display());
    }
  }

  Ok(())
}
