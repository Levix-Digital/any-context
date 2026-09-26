use clap::Parser;
use actx_cli::cli::args::CliArgs;
use actx_cli::cli::oneshot;
use actx_cli::tui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    if args.is_headless() {
        oneshot::run_headless(args).await?;
    } else {
        tui::run_tui(args).await?;
    }

    Ok(())
}
