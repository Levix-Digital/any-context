use clap::Parser;
use actx_cli::cli::args::CliArgs;
use actx_cli::cli::oneshot;
use actx_cli::tui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = std::env::args().collect();
    let mut processed_args: Vec<String> = Vec::with_capacity(raw_args.len() + 2);

    for arg in raw_args {
        if let Some(target) = arg.strip_prefix("--update@") {
            processed_args.push("--update".to_string());
            processed_args.push("--version-target".to_string());
            processed_args.push(target.to_string());
        } else if let Some(target) = arg.strip_prefix("-u@") {
            processed_args.push("--update".to_string());
            processed_args.push("--version-target".to_string());
            processed_args.push(target.to_string());
        } else if let Some(target) = arg.strip_prefix("update@") {
            processed_args.push("update".to_string());
            processed_args.push(target.to_string());
        } else if let Some(target) = arg.strip_prefix("upgrade@") {
            processed_args.push("update".to_string());
            processed_args.push(target.to_string());
        } else if let Some(target) = arg.strip_prefix("--update=") {
            processed_args.push("--update".to_string());
            processed_args.push("--version-target".to_string());
            processed_args.push(target.to_string());
        } else if let Some(target) = arg.strip_prefix("-u=") {
            processed_args.push("--update".to_string());
            processed_args.push("--version-target".to_string());
            processed_args.push(target.to_string());
        } else {
            processed_args.push(arg);
        }
    }

    let args = CliArgs::parse_from(processed_args);

    if args.is_headless() {
        oneshot::run_headless(args).await?;
    } else {
        tui::run_tui(args).await?;
    }

    Ok(())
}
