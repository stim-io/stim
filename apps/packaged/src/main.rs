use std::{
    env,
    process::{exit, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use stim_shared::{
    control_plane::RendererDeliveryLaunchBridge,
    paths::{renderer_delivery_launch_bridge_path, workspace_root},
};
use stim_sidecar::{
    identity::{
        namespace_or_default, SidecarMode, SidecarStamp, SIDECAR_MODE_ENV, SIDECAR_NAMESPACE_ENV,
        SOURCE_APP_PACKAGED,
    },
    ready::{wait_for_ready_line, SidecarReadyLine},
    stamp::{create_stamp_args, read_stamp},
};

const CONTROLLER_BIN_ENV: &str = "STIM_CONTROLLER_BIN";
const CONTROLLER_ENDPOINT_ENV: &str = "STIM_CONTROLLER_ENDPOINT";
const CONTROLLER_INSTANCE_ENV: &str = "STIM_CONTROLLER_INSTANCE_ID";
const CONTROLLER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const RUNNER_READY_TIMEOUT: Duration = Duration::from_secs(120);
const RUN_RENDERER_COMMAND: &str = "__run-renderer";
const RUN_TAURI_COMMAND: &str = "__run-tauri";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PackagedSidecarPlan {
    sidecars: Vec<PackagedSidecarEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct PackagedSidecarEntry {
    stamp: SidecarStamp,
    role: String,
    instance_id: String,
    stamp_args: Vec<String>,
}

fn packaged_sidecar_plan(namespace: Option<&str>) -> PackagedSidecarPlan {
    let namespace = namespace_or_default(namespace);
    let mode = SidecarMode::Runtime;

    PackagedSidecarPlan {
        sidecars: [
            (
                SidecarStamp {
                    app: "renderer".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "renderer-delivery",
                format!("{namespace}-renderer"),
            ),
            (
                SidecarStamp {
                    app: "controller".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "controller-runtime",
                format!("{namespace}-controller"),
            ),
            (
                SidecarStamp {
                    app: "tauri".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "tauri-host",
                format!("{namespace}-tauri"),
            ),
        ]
        .into_iter()
        .map(|(stamp, role, instance_id)| PackagedSidecarEntry {
            stamp_args: create_stamp_args(&stamp),
            stamp,
            role: role.into(),
            instance_id,
        })
        .collect(),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("stim-packaged: {error}");
        exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut namespace: Option<String> = None;
    let mut emit_plan = false;
    let mut launch: Option<String> = None;
    let mut raw_args = env::args().skip(1).collect::<Vec<_>>();
    let first = raw_args.first().cloned();

    match first.as_deref() {
        Some(RUN_RENDERER_COMMAND) => return run_renderer_sidecar(raw_args.split_off(1)),
        Some(RUN_TAURI_COMMAND) => return run_tauri_sidecar(raw_args.split_off(1)),
        None => {}
        Some(_) => {}
    }
    let mut args = raw_args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
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

    let plan = packaged_sidecar_plan(namespace.as_deref());

    if let Some(app) = launch {
        return launch_packaged_sidecar(&plan, &app);
    }

    if emit_plan {
        let output = serde_json::to_string_pretty(&plan)
            .map_err(|error| format!("failed to serialize packaged plan: {error}"))?;
        println!("{output}");
        return Ok(());
    }

    Err("packaged launch requires --plan or launch <controller|renderer|tauri>".into())
}

fn launch_packaged_sidecar(plan: &PackagedSidecarPlan, app: &str) -> Result<(), String> {
    if app == "all" {
        return launch_all(plan);
    }

    let sidecar = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == app)
        .ok_or_else(|| format!("unknown packaged sidecar app: {app}"))?;

    match app {
        "controller" => launch_controller(sidecar),
        "renderer" => launch_runner(RUN_RENDERER_COMMAND, sidecar),
        "tauri" => launch_runner(RUN_TAURI_COMMAND, sidecar),
        _ => Err(format!("unknown packaged sidecar app: {app}")),
    }
}

fn launch_all(plan: &PackagedSidecarPlan) -> Result<(), String> {
    let renderer = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "renderer")
        .ok_or_else(|| "packaged renderer sidecar plan is missing".to_string())?;
    let tauri = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "tauri")
        .ok_or_else(|| "packaged tauri sidecar plan is missing".to_string())?;
    let controller = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "controller")
        .ok_or_else(|| "packaged controller sidecar plan is missing".to_string())?;
    let mut children = Vec::new();
    let mut ready_lines = Vec::new();

    let (renderer_child, renderer_ready) = spawn_runner_ready(RUN_RENDERER_COMMAND, renderer)?;
    children.push((renderer.stamp.app.clone(), renderer_child));
    let renderer_url = renderer_ready
        .endpoint
        .clone()
        .ok_or_else(|| "packaged renderer ready line did not include endpoint".to_string())?;
    write_renderer_delivery_bridge(
        &renderer.stamp.namespace,
        renderer.stamp.mode,
        &renderer_url,
        &renderer.stamp.source,
    )?;
    ready_lines.push(renderer_ready);

    let (controller_child, controller_ready) = spawn_controller_ready(controller)?;
    children.push((controller.stamp.app.clone(), controller_child));
    let controller_endpoint = controller_ready
        .endpoint
        .clone()
        .ok_or_else(|| "packaged controller ready line did not include endpoint".to_string())?;
    let controller_instance_id = controller_ready.instance_id.clone();
    ready_lines.push(controller_ready);

    let (tauri_child, tauri_ready) = spawn_runner_ready_with_env(
        RUN_TAURI_COMMAND,
        tauri,
        &[
            (CONTROLLER_ENDPOINT_ENV, controller_endpoint.as_str()),
            (CONTROLLER_INSTANCE_ENV, controller_instance_id.as_str()),
        ],
    )?;
    children.push((tauri.stamp.app.clone(), tauri_child));
    ready_lines.push(tauri_ready);

    let output = serde_json::to_string(&serde_json::json!({
        "kind": "stim-packaged-ready",
        "sidecars": ready_lines,
    }))
    .map_err(|error| format!("failed to serialize packaged ready lines: {error}"))?;
    println!("{output}");

    let mut first_error: Option<String> = None;
    for (app, mut child) in children {
        match child.wait() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                first_error.get_or_insert_with(|| {
                    format!("packaged {app} runner exited with status {status}")
                });
            }
            Err(error) => {
                first_error
                    .get_or_insert_with(|| format!("failed waiting for packaged {app}: {error}"));
            }
        }
    }

    if let Some(error) = first_error {
        Err(error)
    } else {
        Ok(())
    }
}

fn launch_controller(sidecar: &PackagedSidecarEntry) -> Result<(), String> {
    let (mut child, ready) = spawn_controller_ready(sidecar)?;

    let output = serde_json::to_string(&ready)
        .map_err(|error| format!("failed to serialize packaged controller ready line: {error}"))?;
    println!("{output}");

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for packaged controller sidecar: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "packaged controller sidecar exited with status {status}"
        ))
    }
}

fn spawn_controller_ready(
    sidecar: &PackagedSidecarEntry,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let mut command = controller_command(&sidecar.stamp_args);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn packaged controller sidecar: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "packaged controller sidecar stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, CONTROLLER_READY_TIMEOUT)
        .map_err(|error| format!("packaged controller ready failed: {error}"))?;

    validate_ready(sidecar, &ready)?;

    Ok((child, ready))
}

fn launch_runner(command_name: &str, sidecar: &PackagedSidecarEntry) -> Result<(), String> {
    let (mut child, ready) = spawn_runner_ready(command_name, sidecar)?;

    let output = serde_json::to_string(&ready).map_err(|error| {
        format!(
            "failed to serialize packaged {} ready line: {error}",
            sidecar.stamp.app
        )
    })?;
    println!("{output}");

    let status = child.wait().map_err(|error| {
        format!(
            "failed waiting for packaged {} runner: {error}",
            sidecar.stamp.app
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "packaged {} runner exited with status {status}",
            sidecar.stamp.app
        ))
    }
}

fn spawn_runner_ready(
    command_name: &str,
    sidecar: &PackagedSidecarEntry,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_runner_ready_with_env(command_name, sidecar, &[])
}

fn spawn_runner_ready_with_env(
    command_name: &str,
    sidecar: &PackagedSidecarEntry,
    envs: &[(&str, &str)],
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("failed to resolve current stim-packaged executable: {error}"))?;
    let mut command = Command::new(executable);
    command
        .arg(command_name)
        .args(&sidecar.stamp_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().map_err(|error| {
        format!(
            "failed to spawn packaged {} runner: {error}",
            sidecar.stamp.app
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("packaged {} runner stdout was not piped", sidecar.stamp.app))?;
    let ready = wait_for_ready_line(stdout, RUNNER_READY_TIMEOUT)
        .map_err(|error| format!("packaged {} ready failed: {error}", sidecar.stamp.app))?;

    validate_ready(sidecar, &ready)?;

    Ok((child, ready))
}

fn validate_ready(sidecar: &PackagedSidecarEntry, ready: &SidecarReadyLine) -> Result<(), String> {
    if !ready.is_ready_line() {
        return Err(format!(
            "packaged {} emitted an unexpected ready line",
            sidecar.stamp.app
        ));
    }

    if ready.stamp != sidecar.stamp {
        return Err(format!(
            "packaged {} ready stamp did not match launch stamp",
            sidecar.stamp.app
        ));
    }

    if ready.role != sidecar.role {
        return Err(format!(
            "packaged {} ready role did not match expected {}: {}",
            sidecar.stamp.app, sidecar.role, ready.role
        ));
    }

    Ok(())
}

fn controller_command(stamp_args: &[String]) -> Command {
    if let Ok(binary) = env::var(CONTROLLER_BIN_ENV) {
        let mut command = Command::new(binary);

        command.arg("serve").args(stamp_args);
        return command;
    }

    let mut command = Command::new("cargo");

    command
        .args(["run", "-p", "stim-controller", "--", "serve"])
        .args(stamp_args)
        .current_dir(workspace_root());

    command
}

fn run_renderer_sidecar(args: Vec<String>) -> Result<(), String> {
    let stamp = read_stamp(&args).map_err(|error| format!("invalid renderer stamp: {error}"))?;
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(["run", "-p", "stim-renderer", "--", "serve", "--runtime"])
        .args(&stamp_args)
        .current_dir(workspace_root())
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn packaged renderer server: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "packaged renderer server stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, RUNNER_READY_TIMEOUT)
        .map_err(|error| format!("packaged renderer server ready failed: {error}"))?;
    let expected = PackagedSidecarEntry {
        stamp: stamp.clone(),
        role: "renderer-delivery".into(),
        instance_id: String::new(),
        stamp_args,
    };
    validate_ready(&expected, &ready)?;
    let output = serde_json::to_string(&ready)
        .map_err(|error| format!("failed to serialize renderer ready line: {error}"))?;
    println!("{output}");

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for packaged renderer server: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "packaged renderer server exited with status {status}"
        ))
    }
}

fn run_tauri_sidecar(args: Vec<String>) -> Result<(), String> {
    let stamp = read_stamp(&args).map_err(|error| format!("invalid tauri stamp: {error}"))?;
    let mut child = Command::new("cargo")
        .args(["run", "--no-default-features", "--"])
        .current_dir(workspace_root().join("apps/tauri/src-tauri"))
        .env(SIDECAR_NAMESPACE_ENV, &stamp.namespace)
        .env(SIDECAR_MODE_ENV, stamp.mode.as_str())
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn packaged tauri host: {error}"))?;
    let ready = SidecarReadyLine::new(
        stamp,
        "tauri-host".into(),
        format!("tauri-{}", timestamp_now()),
        None,
        timestamp_now(),
    );
    let output = serde_json::to_string(&ready)
        .map_err(|error| format!("failed to serialize tauri ready line: {error}"))?;
    println!("{output}");

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for packaged tauri host: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("packaged tauri host exited with status {status}"))
    }
}

fn print_help() {
    println!(
        "stim-packaged commands:\n  --plan [--namespace <value>]             print packaged runtime sidecar assembly plan\n  launch all [--namespace <value>]         run packaged renderer delivery and Tauri host\n  launch controller [--namespace <value>]  run packaged controller sidecar in the foreground\n  launch renderer [--namespace <value>]    build and hold packaged renderer delivery\n  launch tauri [--namespace <value>]       run packaged Tauri host in the foreground"
    );
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn write_renderer_delivery_bridge(
    namespace: &str,
    mode: SidecarMode,
    renderer_url: &str,
    source: &str,
) -> Result<(), String> {
    let bridge = RendererDeliveryLaunchBridge {
        namespace: namespace.into(),
        renderer_url: renderer_url.into(),
        source: source.into(),
        published_at: timestamp_now(),
    };
    let path = renderer_delivery_launch_bridge_path(mode.as_str(), namespace);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create renderer delivery bridge dir: {error}"))?;
    }
    let body = serde_json::to_string_pretty(&bridge)
        .map_err(|error| format!("failed to serialize renderer delivery bridge: {error}"))?;
    std::fs::write(&path, format!("{body}\n"))
        .map_err(|error| format!("failed to write renderer delivery bridge: {error}"))
}

#[cfg(test)]
mod tests {
    use super::packaged_sidecar_plan;
    use stim_sidecar::identity::SidecarMode;

    #[test]
    fn packaged_plan_models_runtime_sidecars() {
        let plan = packaged_sidecar_plan(Some("default"));

        assert_eq!(plan.sidecars.len(), 3);
        assert!(plan
            .sidecars
            .iter()
            .all(|sidecar| sidecar.stamp.mode == SidecarMode::Runtime));
        assert_eq!(
            plan.sidecars
                .iter()
                .map(|sidecar| sidecar.stamp.app.as_str())
                .collect::<Vec<_>>(),
            vec!["renderer", "controller", "tauri"]
        );
        assert_eq!(
            plan.sidecars
                .iter()
                .map(|sidecar| sidecar.stamp.source.as_str())
                .collect::<Vec<_>>(),
            vec!["app:packaged", "app:packaged", "app:packaged"]
        );
        assert!(plan.sidecars.iter().all(|sidecar| sidecar
            .stamp_args
            .iter()
            .any(|arg| arg == "--stim-stamp-mode=runtime")));
        assert!(plan.sidecars.iter().all(|sidecar| sidecar
            .stamp_args
            .iter()
            .all(|arg| !arg.starts_with("--stim-stamp-role")
                && !arg.starts_with("--stim-stamp-instance"))));
    }
}
