use std::path::PathBuf;

use crate::cli::{help_text, parse_args, CliCommand, OutputFormat};

#[test]
fn parses_check_options() {
    let CliCommand::Check(options) = parse_args(vec![
        "check".into(),
        "--root".into(),
        "workspace".into(),
        "--format=json".into(),
        "--strict-warnings".into(),
    ])
    .unwrap() else {
        panic!("expected check command");
    };

    assert_eq!(options.root, PathBuf::from("workspace"));
    assert_eq!(options.format, OutputFormat::Json);
    assert!(options.strict_warnings);
}

#[test]
fn command_can_be_omitted() {
    let CliCommand::Check(options) = parse_args(Vec::new()).unwrap() else {
        panic!("expected check command");
    };

    assert_eq!(options.root, PathBuf::from("."));
    assert_eq!(options.format, OutputFormat::Text);
    assert!(!options.strict_warnings);
}

#[test]
fn help_command_is_parsed() {
    assert_eq!(parse_args(vec!["help".into()]).unwrap(), CliCommand::Help);
}

#[test]
fn help_stays_operational() {
    let help = help_text();

    assert!(help.contains("check [--root <path>]"));
    assert!(help.contains("rule-level bad-flavor notes"));
    assert!(!help.contains("Preferred repair"));
    assert!(help.contains("does not format, rewrite"));
}

#[test]
fn json_format_available() {
    let CliCommand::Check(options) =
        parse_args(vec!["check".into(), "--format".into(), "json".into()]).unwrap()
    else {
        panic!("expected check command");
    };

    assert_eq!(options.format, OutputFormat::Json);
}
