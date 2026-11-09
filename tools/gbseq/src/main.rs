use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about = "plita track-based Game Boy sequencer")]
struct Cli {
    /// Exercise the native window and GPU loop without audio output (CI).
    #[arg(long, hide = true)]
    startup_smoke: Option<PathBuf>,
    /// Open a project directly in the graphical editor.
    #[arg(long)]
    open: Option<PathBuf>,
    /// Display language (otherwise saved preference / OS language).
    #[arg(long, value_parser = ["ja", "en"])]
    language: Option<String>,
    #[command(subcommand)]
    command: Option<gbseq::cli::Command>,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    if let Some(path) = args.startup_smoke {
        return gbseq::app::run_startup_smoke(path).map_err(Into::into);
    }
    match args.command {
        Some(command) => command.run(),
        None => gbseq::app::run_gui_with_language(args.open, args.language).map_err(Into::into),
    }
}
