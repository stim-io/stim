#![allow(dead_code)]

#[path = "../src/cli.rs"]
mod cli;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/naming/mod.rs"]
mod naming;
#[path = "../src/output.rs"]
mod output;
#[path = "../src/path_match.rs"]
mod path_match;
#[path = "../src/rust_tests.rs"]
mod rust_tests;
#[path = "../src/scan.rs"]
mod scan;

#[path = "unit/cli.rs"]
mod cli_cases;
#[path = "unit/model.rs"]
mod model_cases;
#[path = "unit/naming.rs"]
mod naming_cases;
#[path = "unit/output.rs"]
mod output_cases;
#[path = "unit/path_match.rs"]
mod path_cases;
#[path = "unit/scan.rs"]
mod scan_cases;
