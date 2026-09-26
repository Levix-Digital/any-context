use std::env;
use actx_installer::run_launcher_or_update_workflow;

fn main() {
    let args: Vec<String> = env::args().collect();
    let current_exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("actx"));
    run_launcher_or_update_workflow(&args, &current_exe);
}
