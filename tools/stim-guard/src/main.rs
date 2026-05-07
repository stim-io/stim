use std::{env, process::exit};

mod cli;
mod config;
mod model;
mod naming;
mod output;
mod path_match;
mod rust_tests;
mod scan;

use cli::{help_text, parse_args, CliCommand};
use config::GuardConfig;
use model::Report;
use output::print_report;
use scan::run_checks;

fn main() {
    match run() {
        Ok(exit_code) => exit(exit_code),
        Err(error) => {
            eprintln!("stim-guard: {error}");
            exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let options = parse_args(env::args().skip(1).collect())?;
    let CliCommand::Check(options) = options else {
        println!("{}", help_text());
        return Ok(0);
    };
    let config = GuardConfig::core(options.root);
    let issues = run_checks(&config)?;
    let report = Report::new(config.root, issues);
    let deny_count = report.deny_count();
    let warning_count = report.warning_count();

    print_report(&report, options.format)?;

    if deny_count > 0 || (options.strict_warnings && warning_count > 0) {
        return Ok(1);
    }
    Ok(0)
}
