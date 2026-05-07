use crate::cli::{help_text, parse_command_line, reject_extra_args};

#[test]
fn namespace_parses_as_option() {
    let (namespace, command, args) =
        parse_command_line(vec!["list".into(), "--namespace".into(), "dev-a".into()]).unwrap();

    assert_eq!(namespace.as_deref(), Some("dev-a"));
    assert_eq!(command, "list");
    assert!(args.is_empty());

    assert!(reject_extra_args(vec!["dev-a".into()], "list")
        .unwrap_err()
        .contains("--namespace <value>"));
}

#[test]
fn namespace_precedes_command() {
    let (namespace, command, args) = parse_command_line(vec![
        "--namespace=dev-b".into(),
        "start".into(),
        "renderer".into(),
    ])
    .unwrap();

    assert_eq!(namespace.as_deref(), Some("dev-b"));
    assert_eq!(command, "start");
    assert_eq!(args, vec!["renderer"]);
}

#[test]
fn omitted_namespace_falls_back() {
    let (namespace, command, args) = parse_command_line(vec!["list".into()]).unwrap();

    assert_eq!(namespace, None);
    assert_eq!(command, "list");
    assert!(args.is_empty());
}

#[test]
fn help_lists_detect_command() {
    assert!(help_text().contains("\n  detect\n"));
}

#[test]
fn help_lists_renderer_smoke() {
    assert!(help_text().contains("smoke renderer messaging [text]"));
}

#[test]
fn help_lists_chat_harness() {
    assert!(help_text().contains("chat run [--new] [--turn <text>] [text]"));
    assert!(help_text().contains("chat inspect [run_id]"));
}

#[test]
fn help_lists_continuation_smoke() {
    assert!(help_text().contains("smoke renderer continuation [text]"));
}

#[test]
fn help_lists_messaging_acceptance() {
    assert!(help_text().contains("accept controller messaging [text]"));
}

#[test]
fn help_lists_tool_acceptance() {
    assert!(help_text().contains("accept controller tool-activity [text]"));
}

#[test]
fn help_lists_participant_acceptance() {
    assert!(help_text().contains("accept controller participant-routing [text]"));
}

#[test]
fn help_lists_agents_target() {
    assert!(help_text().contains("start [all|agents|controller|renderer|tauri]"));
    assert!(help_text().contains("agents select <instance_id>"));
    assert!(help_text().contains("agents launch <instance_id>"));
    assert!(help_text().contains("agents stop <instance_id>"));
    assert!(help_text().contains("inspect agents runtime"));
    assert!(help_text().contains("inspect agents instances"));
    assert!(help_text().contains("inspect agents probe <instance_id>"));
    assert!(help_text().contains("inspect agents provider-probe <instance_id>"));
}
