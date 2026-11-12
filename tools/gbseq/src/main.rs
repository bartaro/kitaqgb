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
    #[arg(long, value_parser = ["en","ja","ko","zh-CN","zh-TW","es","pt-BR","pt","fr","de"],ignore_case=true)]
    language: Option<String>,
    #[command(subcommand)]
    command: Option<gbseq::cli::Command>,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    // Set this before creating a window, MIDI connection or audio thread.
    if args.command.is_none() && std::env::var_os("PLITA_UI_FONT_FALLBACK").is_none() {
        let rel = "assets/ui-font/NotoSansCJKjp-Regular.otf";
        let packaged = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(rel)));
        if let Some(path) = packaged
            .into_iter()
            .chain([PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)])
            .find(|p| p.is_file())
        {
            std::env::set_var("PLITA_UI_FONT_FALLBACK", path);
        }
    }
    if let Some(path) = args.startup_smoke {
        return gbseq::app::run_startup_smoke(path).map_err(Into::into);
    }
    match args.command {
        Some(command) => command.run(),
        None => gbseq::app::run_gui_with_language(args.open, args.language).map_err(Into::into),
    }
}
