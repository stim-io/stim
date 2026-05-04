use std::{env, process::exit};

mod acceptance;
mod bridge;
mod cli;
mod clock;
mod detect;
mod dev_loop;
mod runtime_control;
mod sidecars;
mod smoke;

use acceptance::accept;
use cli::{help_text, parse_command_line, parse_start_options, print_help, reject_extra_args};
use detect::detect;
use dev_loop::{restart, start, ExistingInstancePolicy};
use runtime_control::{inspect, list, reset, status, stop};
use smoke::smoke;
use stim_sidecar::identity::SIDECAR_NAMESPACE_ENV;

fn main() {
    if let Err(error) = run() {
        eprintln!("stim-dev: {error}");
        exit(1);
    }
}

fn run() -> Result<(), String> {
    let (namespace, command, args) = parse_command_line(env::args().skip(1).collect())?;
    if let Some(namespace) = namespace {
        env::set_var(SIDECAR_NAMESPACE_ENV, namespace);
    }

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "start" => start(parse_start_options(args)?, ExistingInstancePolicy::Reject),
        "restart" => restart(parse_start_options(args)?),
        "detect" => {
            reject_extra_args(args, "detect")?;
            detect()
        }
        "accept" => accept(args),
        "smoke" => smoke(args),
        "status" => {
            reject_extra_args(args, "status")?;
            status()
        }
        "inspect" => inspect(args),
        "list" => {
            reject_extra_args(args, "list")?;
            list()
        }
        "stop" => {
            reject_extra_args(args, "stop")?;
            stop()
        }
        "reset" => {
            reject_extra_args(args, "reset")?;
            reset()
        }
        other => Err(format!("unsupported command: {other}\n\n{}", help_text())),
    }
}
