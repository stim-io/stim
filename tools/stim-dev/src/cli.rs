use stim_sidecar::identity::namespace_or_default;

#[derive(Clone, Copy)]
pub(crate) enum StartTarget {
    All,
    Controller,
    Renderer,
    Tauri,
}

#[derive(Clone, Copy)]
pub(crate) struct StartOptions {
    pub(crate) target: StartTarget,
}

pub(crate) fn help_text() -> &'static str {
    "stim-dev [--namespace <value>] commands:\n  default namespace is the fallback when --namespace is omitted\n  start [all|controller|renderer|tauri]\n  restart [all|controller|renderer|tauri]\n  detect\n  accept controller messaging [text]\n  smoke renderer messaging [text]\n  smoke renderer continuation [text]\n  status\n  inspect tauri host\n  inspect tauri screenshot [label]\n  inspect renderer landing\n  inspect renderer messaging\n  list\n  stop\n  reset\n  help"
}

pub(crate) fn print_help() {
    println!("{}", help_text());
}

pub(crate) fn parse_command_line(
    args: Vec<String>,
) -> Result<(Option<String>, String, Vec<String>), String> {
    let (namespace, args) = take_namespace_option(args)?;
    let mut args = args.into_iter();
    let command = args.next().unwrap_or_else(|| "start".to_string());
    Ok((namespace, command, args.collect()))
}

fn take_namespace_option(args: Vec<String>) -> Result<(Option<String>, Vec<String>), String> {
    let mut namespace = None;
    let mut rest = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if arg == "--namespace" {
            let value = args
                .next()
                .ok_or_else(|| "--namespace requires a value".to_string())?;
            namespace = Some(namespace_or_default(Some(&value)));
        } else if let Some(value) = arg.strip_prefix("--namespace=") {
            namespace = Some(namespace_or_default(Some(value)));
        } else {
            rest.push(arg);
        }
    }

    Ok((namespace, rest))
}

pub(crate) fn reject_extra_args(args: Vec<String>, command: &str) -> Result<(), String> {
    if let Some(extra) = args.into_iter().next() {
        return Err(format!(
            "unsupported {command} argument: {extra}; pass namespace with --namespace <value>"
        ));
    }
    Ok(())
}

fn parse_start_target(value: Option<&str>) -> Result<StartTarget, String> {
    match value.unwrap_or("all") {
        "all" => Ok(StartTarget::All),
        "controller" => Ok(StartTarget::Controller),
        "renderer" => Ok(StartTarget::Renderer),
        "tauri" => Ok(StartTarget::Tauri),
        other => Err(format!("unsupported start target: {other}")),
    }
}

pub(crate) fn parse_start_options(args: Vec<String>) -> Result<StartOptions, String> {
    let mut target: Option<StartTarget> = None;

    for arg in args {
        if arg.starts_with("--") {
            return Err(format!(
                "unsupported start argument: {arg}; use 'stim-dev restart' for recovery"
            ));
        } else if target.is_none() {
            target = Some(parse_start_target(Some(&arg))?);
        } else {
            return Err(format!("unsupported start argument: {arg}"));
        }
    }

    let target = target.unwrap_or(StartTarget::All);
    Ok(StartOptions { target })
}

#[cfg(test)]
mod tests {
    use super::{parse_command_line, reject_extra_args};

    #[test]
    fn namespace_is_parsed_as_an_option_not_a_positional_namespace() {
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
    fn namespace_option_can_precede_command() {
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
    fn omitted_namespace_uses_fallback_at_runtime() {
        let (namespace, command, args) = parse_command_line(vec!["list".into()]).unwrap();

        assert_eq!(namespace, None);
        assert_eq!(command, "list");
        assert!(args.is_empty());
    }

    #[test]
    fn help_lists_detect_as_a_read_only_diagnostic_command() {
        assert!(super::help_text().contains("\n  detect\n"));
    }

    #[test]
    fn help_lists_renderer_messaging_smoke_as_a_dev_smoke_command() {
        assert!(super::help_text().contains("smoke renderer messaging [text]"));
    }

    #[test]
    fn help_lists_renderer_continuation_smoke_as_a_human_visible_smoke_command() {
        assert!(super::help_text().contains("smoke renderer continuation [text]"));
    }

    #[test]
    fn help_lists_controller_messaging_acceptance_as_machine_gate() {
        assert!(super::help_text().contains("accept controller messaging [text]"));
    }
}
