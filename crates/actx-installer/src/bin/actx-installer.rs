use std::env;
use actx_installer::{run_installer_workflow, run_launcher_or_update_workflow};

fn main() {
    let args: Vec<String> = env::args().collect();
    let current_exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("actx-installer"));

    // If passed --launcher, run in launcher mode, otherwise run as installer
    if args.iter().any(|a| a == "--launcher") {
        run_launcher_or_update_workflow(&args, &current_exe);
    } else {
        run_installer_workflow(&args);
    }
}
