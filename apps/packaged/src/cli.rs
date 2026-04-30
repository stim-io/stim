pub(crate) const RUN_RENDERER_COMMAND: &str = "__run-renderer";
pub(crate) const RUN_TAURI_COMMAND: &str = "__run-tauri";

pub(crate) enum CommandLine {
    Help,
    PrintPlan {
        namespace: Option<String>,
    },
    Launch {
        namespace: Option<String>,
        app: String,
    },
    RunRenderer {
        args: Vec<String>,
    },
    RunTauri {
        args: Vec<String>,
    },
}

pub(crate) fn parse_args(mut raw_args: Vec<String>) -> Result<CommandLine, String> {
    match raw_args.first().map(String::as_str) {
        Some(RUN_RENDERER_COMMAND) => {
            return Ok(CommandLine::RunRenderer {
                args: raw_args.split_off(1),
            });
        }
        Some(RUN_TAURI_COMMAND) => {
            return Ok(CommandLine::RunTauri {
                args: raw_args.split_off(1),
            });
        }
        None | Some(_) => {}
    }

    let mut namespace: Option<String> = None;
    let mut emit_plan = false;
    let mut launch: Option<String> = None;
    let mut args = raw_args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(CommandLine::Help),
            "--plan" => emit_plan = true,
            "launch" => {
                launch = Some(
                    args.next()
                        .ok_or_else(|| "launch requires a sidecar app name".to_string())?,
                );
            }
            "--namespace" => {
                namespace = Some(
                    args.next()
                        .ok_or_else(|| "--namespace requires a value".to_string())?,
                );
            }
            other => return Err(format!("unsupported argument: {other}")),
        }
    }

    if let Some(app) = launch {
        return Ok(CommandLine::Launch { namespace, app });
    }

    if emit_plan {
        return Ok(CommandLine::PrintPlan { namespace });
    }

    Err("packaged launch requires --plan or launch <controller|renderer|tauri>".into())
}

pub(crate) fn print_help() {
    println!(
        "stim-packaged commands:\n  --plan [--namespace <value>]             print packaged runtime sidecar assembly plan\n  launch all [--namespace <value>]         run packaged renderer delivery and Tauri host\n  launch controller [--namespace <value>]  run packaged controller sidecar in the foreground\n  launch renderer [--namespace <value>]    build and hold packaged renderer delivery\n  launch tauri [--namespace <value>]       run packaged Tauri host in the foreground"
    );
}
