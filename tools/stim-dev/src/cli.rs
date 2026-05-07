use stim_sidecar::identity::namespace_or_default;

#[derive(Clone, Copy)]
pub(crate) enum StartTarget {
    All,
    Agents,
    Controller,
    Renderer,
    Tauri,
}

#[derive(Clone, Copy)]
pub(crate) struct StartOptions {
    pub(crate) target: StartTarget,
}

pub(crate) fn help_text() -> &'static str {
    "stim-dev [--namespace <value>] commands:\n  default namespace is the fallback when --namespace is omitted\n  start [all|agents|controller|renderer|tauri]\n  restart [all|agents|controller|renderer|tauri]\n  detect\n  agents select <instance_id>\n  agents launch <instance_id>\n  agents stop <instance_id>\n  agents apply-profile <instance_id> <profile_id>\n  chat run [--new] [--turn <text>] [text]\n  chat inspect [run_id]\n  accept controller messaging [text]\n  accept controller tool-activity [text]\n  accept controller participant-routing [text]\n  smoke renderer messaging [text]\n  smoke renderer continuation [text]\n  status\n  inspect agents runtime\n  inspect agents instances\n  inspect agents profiles\n  inspect agents probe <instance_id>\n  inspect agents provider-probe <instance_id>\n  inspect tauri host\n  inspect tauri screenshot [label]\n  inspect renderer landing\n  inspect renderer messaging\n  list\n  stop\n  reset\n  help"
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
        "agents" => Ok(StartTarget::Agents),
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
