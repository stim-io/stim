use std::{env, process::exit};

mod bridge;
mod cli;
mod clock;
mod launch;
mod plan;
mod runner;

use cli::{parse_args, print_help, CommandLine};
use launch::launch_packaged_sidecar;
use plan::packaged_sidecar_plan;
use runner::{run_renderer_sidecar, run_tauri_sidecar};

fn main() {
    if let Err(error) = run() {
        eprintln!("stim-packaged: {error}");
        exit(1);
    }
}

fn run() -> Result<(), String> {
    match parse_args(env::args().skip(1).collect())? {
        CommandLine::Help => {
            print_help();
            Ok(())
        }
        CommandLine::PrintPlan { namespace } => {
            let plan = packaged_sidecar_plan(namespace.as_deref());
            let output = serde_json::to_string_pretty(&plan)
                .map_err(|error| format!("failed to serialize packaged plan: {error}"))?;

            println!("{output}");
            Ok(())
        }
        CommandLine::Launch { namespace, app } => {
            let plan = packaged_sidecar_plan(namespace.as_deref());
            launch_packaged_sidecar(&plan, &app)
        }
        CommandLine::RunRenderer { args } => run_renderer_sidecar(args),
        CommandLine::RunTauri { args } => run_tauri_sidecar(args),
    }
}
