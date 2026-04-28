use std::{
    env, fs,
    process::{exit, Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use stim_shared::{
    control_plane::RendererDeliveryLaunchBridge,
    inspection::{
        ControllerRuntimeBridgeRequest, ControllerRuntimeBridgeResponse, InspectBridgeRequest,
        InspectBridgeResponse, InspectResult, RendererProbeBridgeRequest,
        RendererProbeBridgeResponse, RendererProbeRequest, RendererProbeResult,
        RendererProbeSnapshot, ScreenshotBridgeRequest, ScreenshotBridgeResponse, ScreenshotResult,
    },
    paths::{
        controller_runtime_bridge_request_path, controller_runtime_bridge_response_path,
        inspect_bridge_request_path, inspect_bridge_response_path,
        renderer_delivery_launch_bridge_path, renderer_probe_bridge_request_path,
        renderer_probe_bridge_response_path, screenshot_bridge_request_path,
        screenshot_bridge_response_path, tauri_app_dir,
    },
};
use stim_sidecar::{
    identity::{
        namespace_or_default, SidecarMode, SidecarStamp, SIDECAR_MODE_ENV, SIDECAR_NAMESPACE_ENV,
        SOURCE_TOOL_STIM_DEV,
    },
    process::StampedProcessCriteria,
    ready::{wait_for_ready_line, SidecarReadyLine},
    stamp::create_stamp_args,
};

#[derive(Clone, Copy)]
enum StartTarget {
    All,
    Controller,
    Renderer,
    Tauri,
}

#[derive(Clone, Copy)]
struct StartOptions {
    target: StartTarget,
}

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

fn help_text() -> &'static str {
    "stim-dev [--namespace <value>] commands:\n  default namespace is the fallback when --namespace is omitted\n  start [all|controller|renderer|tauri]\n  restart [all|controller|renderer|tauri]\n  status\n  inspect tauri host\n  inspect tauri screenshot [label]\n  inspect renderer landing\n  inspect renderer messaging\n  list\n  stop\n  reset\n  help"
}

fn print_help() {
    println!("{}", help_text());
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn create_request_id() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!(
        "{}-{}-{}",
        duration.as_secs(),
        duration.subsec_nanos(),
        std::process::id()
    )
}

fn parse_command_line(args: Vec<String>) -> Result<(Option<String>, String, Vec<String>), String> {
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

fn reject_extra_args(args: Vec<String>, command: &str) -> Result<(), String> {
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

fn parse_start_options(args: Vec<String>) -> Result<StartOptions, String> {
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

enum ExistingInstancePolicy {
    Reject,
    Allow,
}

fn start(options: StartOptions, existing_policy: ExistingInstancePolicy) -> Result<(), String> {
    if matches!(existing_policy, ExistingInstancePolicy::Reject) {
        reject_existing_instance()?;
    }

    match options.target {
        StartTarget::All => start_all(),
        StartTarget::Controller => start_controller(),
        StartTarget::Tauri => start_tauri(),
        StartTarget::Renderer => start_renderer_foreground(false),
    }
}

fn restart(options: StartOptions) -> Result<(), String> {
    let namespace = current_namespace();
    match options.target {
        StartTarget::All => {
            let criteria = StampedProcessCriteria {
                namespace: Some(namespace.clone()),
                ..StampedProcessCriteria::default()
            };
            let _ = stop_matching_processes(&criteria)?;
            restart_all(&namespace)
        }
        StartTarget::Controller => {
            let criteria = StampedProcessCriteria {
                app: Some("controller".into()),
                namespace: Some(namespace.clone()),
                ..StampedProcessCriteria::default()
            };
            let _ = stop_matching_processes(&criteria)?;
            start(options, ExistingInstancePolicy::Allow)
        }
        StartTarget::Renderer => {
            let criteria = StampedProcessCriteria {
                app: Some("renderer".into()),
                namespace: Some(namespace.clone()),
                ..StampedProcessCriteria::default()
            };
            let _ = stop_matching_processes(&criteria)?;
            let renderer_dev_stop = stop_renderer_dev_server_processes()?;
            restart_renderer(&namespace, &renderer_dev_stop)
        }
        StartTarget::Tauri => restart_tauri(&namespace),
    }
}

fn reject_existing_instance() -> Result<(), String> {
    let namespace = current_namespace();
    let processes = stamped_processes_for_namespace(&namespace)?;
    let live_host = request_inspect_with_timeout(Duration::from_secs(2)).is_ok();
    let live_controller = request_controller_runtime_with_timeout(Duration::from_secs(2)).is_ok();

    if processes.is_empty() && !live_host && !live_controller {
        return Ok(());
    }

    Err(format!(
        "existing stim-dev instance detected for namespace '{namespace}'; run 'stim-dev stop' before starting again, or use 'stim-dev restart' for an explicit restart"
    ))
}

fn start_tauri() -> Result<(), String> {
    let namespace = current_namespace();
    write_renderer_delivery_bridge(
        &namespace,
        SidecarMode::Dev,
        stim_shared::RENDERER_DEV_URL,
        SOURCE_TOOL_STIM_DEV,
    )?;
    run_tauri_foreground(&namespace, &[])
}

fn current_namespace() -> String {
    namespace_or_default(std::env::var(SIDECAR_NAMESPACE_ENV).ok().as_deref())
}

fn renderer_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "renderer".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn controller_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "controller".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn tauri_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "tauri".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn start_renderer_foreground(force: bool) -> Result<(), String> {
    let namespace = current_namespace();
    let mut args = vec![
        "run".to_string(),
        "-p".into(),
        "stim-renderer".into(),
        "--".into(),
        "serve".into(),
        "--dev".into(),
    ];
    if force {
        args.push("--force".into());
    }
    args.extend(create_stamp_args(&renderer_stamp(&namespace)));

    run_cargo_owned(&stim_platform::paths::workspace_root(), &args)
}

fn restart_renderer(
    namespace: &str,
    renderer_dev_stop: &stim_platform::process::StopProcessResult,
) -> Result<(), String> {
    let (_renderer_child, renderer_ready) = spawn_renderer_ready_detached(namespace, true)?;
    let renderer_url = renderer_ready
        .endpoint
        .clone()
        .ok_or_else(|| "renderer ready line did not include endpoint".to_string())?;
    write_renderer_delivery_bridge(
        namespace,
        SidecarMode::Dev,
        &renderer_url,
        SOURCE_TOOL_STIM_DEV,
    )?;

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "app": "renderer",
        "state": "ready",
        "endpoint": renderer_url,
        "instance_id": renderer_ready.instance_id,
        "unstamped_renderer_dev_cleanup": {
            "already_stopped": renderer_dev_stop.already_stopped,
            "matched_pids": renderer_dev_stop.matched_pids,
            "stopped_pids": renderer_dev_stop.stopped_pids,
            "forced_pids": renderer_dev_stop.forced_pids,
            "remaining_pids": renderer_dev_stop.remaining_pids,
        },
    }))
    .map_err(|error| format!("failed to serialize renderer restart result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn restart_all(namespace: &str) -> Result<(), String> {
    let renderer_dev_stop = stop_renderer_dev_server_processes()?;
    let host_stop = stop_tauri_host_processes()?;

    let (_renderer_child, renderer_ready) = spawn_renderer_ready_detached(namespace, true)?;
    let renderer_url = renderer_ready
        .endpoint
        .clone()
        .ok_or_else(|| "renderer ready line did not include endpoint".to_string())?;
    write_renderer_delivery_bridge(
        namespace,
        SidecarMode::Dev,
        &renderer_url,
        SOURCE_TOOL_STIM_DEV,
    )?;

    let (_controller_child, controller_ready) = spawn_controller_ready_detached(namespace)?;
    let controller_endpoint = controller_ready
        .endpoint
        .clone()
        .ok_or_else(|| "controller ready line did not include endpoint".to_string())?;
    let controller_instance_id = controller_ready.instance_id.clone();

    let _tauri_child = spawn_tauri_detached(
        namespace,
        &[
            ("STIM_CONTROLLER_ENDPOINT", controller_endpoint.as_str()),
            (
                "STIM_CONTROLLER_INSTANCE_ID",
                controller_instance_id.as_str(),
            ),
        ],
    )?;
    let _ = request_inspect_with_timeout(Duration::from_secs(15))?;
    let renderer_landing = require_renderer_landing()?;

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "app": "all",
        "state": "ready",
        "renderer_endpoint": renderer_url,
        "renderer_landing": renderer_landing,
        "controller_endpoint": controller_endpoint,
        "controller_instance_id": controller_instance_id,
        "unstamped_renderer_dev_cleanup": {
            "already_stopped": renderer_dev_stop.already_stopped,
            "matched_pids": renderer_dev_stop.matched_pids,
            "stopped_pids": renderer_dev_stop.stopped_pids,
            "forced_pids": renderer_dev_stop.forced_pids,
            "remaining_pids": renderer_dev_stop.remaining_pids,
        },
        "host_cleanup": {
            "already_stopped": host_stop.already_stopped,
            "matched_pids": host_stop.matched_pids,
            "stopped_pids": host_stop.stopped_pids,
            "forced_pids": host_stop.forced_pids,
            "remaining_pids": host_stop.remaining_pids,
        },
    }))
    .map_err(|error| format!("failed to serialize restart result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn restart_tauri(namespace: &str) -> Result<(), String> {
    let controller_runtime = request_controller_runtime_with_timeout(Duration::from_secs(5))?;
    let controller_endpoint = controller_runtime
        .snapshot
        .http_base_url
        .ok_or_else(|| "controller runtime did not include http_base_url".to_string())?;
    let controller_instance_id = controller_runtime.snapshot.instance_id;
    let host_stop = stop_tauri_host_processes()?;
    write_renderer_delivery_bridge(
        namespace,
        SidecarMode::Dev,
        stim_shared::RENDERER_DEV_URL,
        SOURCE_TOOL_STIM_DEV,
    )?;
    let _tauri_child = spawn_tauri_detached(
        namespace,
        &[
            ("STIM_CONTROLLER_ENDPOINT", controller_endpoint.as_str()),
            (
                "STIM_CONTROLLER_INSTANCE_ID",
                controller_instance_id.as_str(),
            ),
        ],
    )?;
    let _ = request_inspect_with_timeout(Duration::from_secs(15))?;
    let renderer_landing = require_renderer_landing()?;

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "app": "tauri",
        "state": "ready",
        "renderer_landing": renderer_landing,
        "host_cleanup": {
            "already_stopped": host_stop.already_stopped,
            "matched_pids": host_stop.matched_pids,
            "stopped_pids": host_stop.stopped_pids,
            "forced_pids": host_stop.forced_pids,
            "remaining_pids": host_stop.remaining_pids,
        },
    }))
    .map_err(|error| format!("failed to serialize tauri restart result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn start_all() -> Result<(), String> {
    let namespace = current_namespace();
    let mut children = Vec::new();

    let (renderer_child, renderer_ready) = spawn_renderer_ready(&namespace, false)?;
    let renderer_url = renderer_ready
        .endpoint
        .clone()
        .ok_or_else(|| "renderer ready line did not include endpoint".to_string())?;
    write_renderer_delivery_bridge(
        &namespace,
        SidecarMode::Dev,
        &renderer_url,
        SOURCE_TOOL_STIM_DEV,
    )?;
    children.push(("renderer".to_string(), renderer_child));

    let (controller_child, controller_ready) = spawn_controller_ready(&namespace)?;
    let controller_endpoint = controller_ready
        .endpoint
        .clone()
        .ok_or_else(|| "controller ready line did not include endpoint".to_string())?;
    let controller_instance_id = controller_ready.instance_id.clone();
    children.push(("controller".to_string(), controller_child));

    let tauri_child = spawn_tauri(
        &namespace,
        &[
            ("STIM_CONTROLLER_ENDPOINT", controller_endpoint.as_str()),
            (
                "STIM_CONTROLLER_INSTANCE_ID",
                controller_instance_id.as_str(),
            ),
        ],
    )?;
    children.push(("tauri".to_string(), tauri_child));

    wait_children(children)
}

fn start_controller() -> Result<(), String> {
    let namespace = current_namespace();
    let stamp = controller_stamp(&namespace);
    let mut args = vec![
        "run".to_string(),
        "-p".into(),
        "stim-controller".into(),
        "--".into(),
        "serve".into(),
    ];

    args.extend(create_stamp_args(&stamp));

    run_cargo_owned(&stim_platform::paths::workspace_root(), &args)
}

fn spawn_renderer_ready(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_ready_with_stdio(namespace, force, false)
}

fn spawn_renderer_ready_detached(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_ready_with_stdio(namespace, force, true)
}

fn spawn_renderer_ready_with_stdio(
    namespace: &str,
    force: bool,
    detached_stdio: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let mut args = vec!["run", "-p", "stim-renderer", "--", "serve", "--dev"];
    if force {
        args.push("--force");
    }
    let stamp = renderer_stamp(namespace);
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(args)
        .args(&stamp_args)
        .current_dir(stim_platform::paths::workspace_root())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn renderer delivery: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "renderer delivery stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, Duration::from_secs(120))
        .map_err(|error| format!("renderer delivery ready failed: {error}"))?;
    validate_ready(&stamp, "renderer-delivery", &ready)?;
    Ok((child, ready))
}

fn spawn_controller_ready(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_ready_with_stdio(namespace, false)
}

fn spawn_controller_ready_detached(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_ready_with_stdio(namespace, true)
}

fn spawn_controller_ready_with_stdio(
    namespace: &str,
    detached_stdio: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let stamp = controller_stamp(namespace);
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(["run", "-p", "stim-controller", "--", "serve"])
        .args(&stamp_args)
        .current_dir(stim_platform::paths::workspace_root())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn controller: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "controller stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, Duration::from_secs(120))
        .map_err(|error| format!("controller ready failed: {error}"))?;
    validate_ready(&stamp, "controller-runtime", &ready)?;
    Ok((child, ready))
}

fn validate_ready(
    stamp: &SidecarStamp,
    role: &str,
    ready: &SidecarReadyLine,
) -> Result<(), String> {
    if !ready.is_ready_line() || &ready.stamp != stamp || ready.role != role {
        return Err(format!("unexpected {role} ready line"));
    }
    Ok(())
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
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create renderer delivery bridge dir: {error}"))?;
    }
    let body = serde_json::to_string_pretty(&bridge)
        .map_err(|error| format!("failed to serialize renderer delivery bridge: {error}"))?;
    fs::write(&path, format!("{body}\n"))
        .map_err(|error| format!("failed to write renderer delivery bridge: {error}"))
}

fn run_tauri_foreground(namespace: &str, envs: &[(&str, &str)]) -> Result<(), String> {
    let mut child = spawn_tauri(namespace, envs)?;
    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for Tauri host: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Tauri host exited with status {status}"))
    }
}

fn spawn_tauri(namespace: &str, envs: &[(&str, &str)]) -> Result<std::process::Child, String> {
    spawn_tauri_with_stdio(namespace, envs, false)
}

fn spawn_tauri_detached(
    namespace: &str,
    envs: &[(&str, &str)],
) -> Result<std::process::Child, String> {
    spawn_tauri_with_stdio(namespace, envs, true)
}

fn spawn_tauri_with_stdio(
    namespace: &str,
    envs: &[(&str, &str)],
    detached_stdio: bool,
) -> Result<std::process::Child, String> {
    let mut command = Command::new("cargo");
    let stamp_args = create_stamp_args(&tauri_stamp(namespace));
    command
        .args(["run", "--no-default-features", "--"])
        .args(&stamp_args)
        .current_dir(tauri_app_dir().join("src-tauri"))
        .env(SIDECAR_NAMESPACE_ENV, namespace)
        .env(SIDECAR_MODE_ENV, SidecarMode::Dev.as_str())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    for (key, value) in envs {
        command.env(key, value);
    }
    command
        .spawn()
        .map_err(|error| format!("failed to spawn Tauri host: {error}"))
}

fn wait_children(children: Vec<(String, std::process::Child)>) -> Result<(), String> {
    let mut first_error = None;
    for (name, mut child) in children {
        match child.wait() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                first_error.get_or_insert_with(|| format!("{name} exited with status {status}"));
            }
            Err(error) => {
                first_error.get_or_insert_with(|| format!("failed waiting for {name}: {error}"));
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn screenshot(label: Option<String>) -> Result<(), String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = ScreenshotBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        label,
    };

    let request_path = screenshot_bridge_request_path(&request_id);
    let response_path = screenshot_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create screenshot request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create screenshot response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize screenshot request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write screenshot request: {error}"))?;

    let started = SystemTime::now();
    let timeout = Duration::from_secs(15);

    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for screenshot response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<ScreenshotBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse screenshot response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);

                match response.result {
                    ScreenshotResult::Success { path, .. } => {
                        println!("{path}");
                        return Ok(());
                    }
                    ScreenshotResult::Failure { reason } => {
                        return Err(format!("screenshot failed: {:?}", reason));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn inspect(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [app, subcommand] if app == "tauri" && subcommand == "host" => inspect_host(),
        [app, subcommand] if app == "renderer" && subcommand == "landing" => {
            inspect_renderer(RendererProbeRequest::LandingBasics)
        }
        [app, subcommand] if app == "renderer" && subcommand == "messaging" => {
            inspect_renderer(RendererProbeRequest::MessagingState)
        }
        [app, subcommand] if app == "tauri" && subcommand == "screenshot" => {
            inspect_screenshot(None)
        }
        [app, subcommand, label] if app == "tauri" && subcommand == "screenshot" => {
            inspect_screenshot(Some(label.clone()))
        }
        [] | [_] => Err("inspect requires '<app> <subcommand>'; supported leaves: tauri host, tauri screenshot [label], renderer landing, renderer messaging".into()),
        [app, ..] => Err(format!(
            "unsupported inspect leaf under app '{app}'; supported leaves: tauri host, tauri screenshot [label], renderer landing, renderer messaging"
        )),
    }
}

fn inspect_host() -> Result<(), String> {
    match request_inspect()? {
        InspectResult::Success { snapshot } => {
            let output = serde_json::to_string_pretty(&snapshot)
                .map_err(|error| format!("failed to serialize inspect snapshot: {error}"))?;
            println!("{output}");
            Ok(())
        }
        InspectResult::Failure { reason } => Err(format!("inspect failed: {:?}", reason)),
    }
}

fn inspect_renderer(probe: RendererProbeRequest) -> Result<(), String> {
    match request_probe(probe)? {
        RendererProbeResult::Success { snapshot } => {
            let output = serde_json::to_string_pretty(&snapshot).map_err(|error| {
                format!("failed to serialize renderer inspect snapshot: {error}")
            })?;
            println!("{output}");
            Ok(())
        }
        RendererProbeResult::Failure { reason } => {
            Err(format!("renderer inspect failed: {:?}", reason))
        }
    }
}

fn inspect_screenshot(label: Option<String>) -> Result<(), String> {
    screenshot(label)
}

fn status() -> Result<(), String> {
    let namespace = current_namespace();
    let processes = stamped_processes_for_namespace(&namespace)?;
    let host = request_inspect_with_timeout(Duration::from_secs(15));
    let controller_runtime = request_controller_runtime_with_timeout(Duration::from_secs(15));
    let renderer_landing = request_probe(RendererProbeRequest::LandingBasics);

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "checked_at": timestamp_now(),
        "host": bridge_result_json(host),
        "controller_runtime": bridge_result_json(controller_runtime),
        "renderer_landing": renderer_probe_result_json(renderer_landing),
        "stamped_processes": process_list_json(&processes),
    }))
    .map_err(|error| format!("failed to serialize status output: {error}"))?;

    println!("{output}");
    Ok(())
}

fn list() -> Result<(), String> {
    let namespace = current_namespace();
    let processes = stamped_processes_for_namespace(&namespace)?;
    let host = request_inspect_with_timeout(Duration::from_secs(2));
    let controller_runtime = request_controller_runtime_with_timeout(Duration::from_secs(2));
    let renderer_landing = request_probe(RendererProbeRequest::LandingBasics);
    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "live": {
            "host": bridge_result_json(host),
            "controller_runtime": bridge_result_json(controller_runtime),
            "renderer_landing": renderer_probe_result_json(renderer_landing),
        },
        "stamped_processes": process_list_json(&processes),
    }))
    .map_err(|error| format!("failed to serialize process list: {error}"))?;

    println!("{output}");
    Ok(())
}

fn stop() -> Result<(), String> {
    let namespace = current_namespace();
    let result = stop_namespace_processes(&namespace)?;
    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "already_stopped": result.already_stopped,
        "matched_pids": result.matched_pids,
        "stopped_pids": result.stopped_pids,
        "forced_pids": result.forced_pids,
        "remaining_pids": result.remaining_pids,
    }))
    .map_err(|error| format!("failed to serialize stop result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn reset() -> Result<(), String> {
    let namespace = current_namespace();
    let stop_result = stop_namespace_processes(&namespace)?;
    let layout = stim_sidecar::layout::SidecarLayout::new(
        stim_platform::paths::dev_root(),
        Some(&namespace),
    );
    let removed = [
        layout.logs_root.as_path(),
        layout.bridges_root.as_path(),
        layout.locks_root.as_path(),
    ]
    .into_iter()
    .filter_map(|path| remove_tree_if_exists(path).transpose())
    .collect::<Result<Vec<_>, _>>()?;
    let _ = fs::remove_dir(&layout.root);
    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": namespace,
        "stop": {
            "already_stopped": stop_result.already_stopped,
            "matched_pids": stop_result.matched_pids,
            "stopped_pids": stop_result.stopped_pids,
            "forced_pids": stop_result.forced_pids,
            "remaining_pids": stop_result.remaining_pids,
        },
        "removed": removed,
    }))
    .map_err(|error| format!("failed to serialize reset result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn stop_namespace_processes(
    namespace: &str,
) -> Result<stim_platform::process::StopProcessResult, String> {
    let criteria = StampedProcessCriteria {
        namespace: Some(namespace.to_string()),
        ..StampedProcessCriteria::default()
    };
    stop_matching_processes(&criteria)
}

fn stop_matching_processes(
    criteria: &StampedProcessCriteria,
) -> Result<stim_platform::process::StopProcessResult, String> {
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let matched = stim_sidecar::process::matching_stamped_processes(&processes, &criteria);
    let root_pids = matched
        .iter()
        .map(|process| process.pid)
        .collect::<Vec<_>>();
    let tree_pids = stim_platform::process::collect_process_tree_pids(&processes, &root_pids);

    stim_platform::process::stop_processes(&tree_pids)
        .map_err(|error| format!("failed to stop stamped processes: {error}"))
}

fn stop_renderer_dev_server_processes() -> Result<stim_platform::process::StopProcessResult, String>
{
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let matched_pids = processes
        .iter()
        .filter(|process| command_is_renderer_dev_server(&process.command))
        .map(|process| process.pid)
        .collect::<Vec<_>>();

    stim_platform::process::stop_processes(&matched_pids)
        .map_err(|error| format!("failed to stop renderer dev server processes: {error}"))
}

fn stop_tauri_host_processes() -> Result<stim_platform::process::StopProcessResult, String> {
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let matched_pids = processes
        .iter()
        .filter(|process| command_is_tauri_host(&process.command))
        .map(|process| process.pid)
        .collect::<Vec<_>>();

    stim_platform::process::stop_processes(&matched_pids)
        .map_err(|error| format!("failed to stop Tauri host processes: {error}"))
}

fn command_is_tauri_host(command: &str) -> bool {
    let tauri_binary = stim_platform::paths::workspace_root()
        .join("target")
        .join("debug")
        .join("stim-tauri");
    command.contains(tauri_binary.to_string_lossy().as_ref())
}

fn command_is_renderer_dev_server(command: &str) -> bool {
    let renderer_vite_dir = stim_shared::paths::renderer_vite_dir();
    let renderer_vite_dir = renderer_vite_dir.to_string_lossy();

    command.contains(renderer_vite_dir.as_ref())
        && command.contains("vite")
        && command.contains("--host 127.0.0.1")
        && command.contains("--port 1420")
}

fn stamped_processes_for_namespace(
    namespace: &str,
) -> Result<Vec<stim_platform::process::ProcessSnapshot>, String> {
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let criteria = StampedProcessCriteria {
        namespace: Some(namespace.to_string()),
        ..StampedProcessCriteria::default()
    };
    Ok(stim_sidecar::process::matching_stamped_processes(
        &processes, &criteria,
    ))
}

fn process_list_json(processes: &[stim_platform::process::ProcessSnapshot]) -> serde_json::Value {
    serde_json::Value::Array(
        processes
            .iter()
            .map(|process| {
                serde_json::json!({
                    "pid": process.pid,
                    "ppid": process.ppid,
                    "command": process.command,
                })
            })
            .collect(),
    )
}

fn bridge_result_json<T: serde::Serialize>(result: Result<T, String>) -> serde_json::Value {
    match result {
        Ok(value) => serde_json::json!({ "state": "available", "value": value }),
        Err(error) => serde_json::json!({ "state": "unavailable", "detail": error }),
    }
}

fn renderer_probe_result_json(result: Result<RendererProbeResult, String>) -> serde_json::Value {
    match result {
        Ok(RendererProbeResult::Success { snapshot }) => {
            serde_json::json!({ "state": "available", "value": snapshot })
        }
        Ok(RendererProbeResult::Failure { reason }) => serde_json::json!({
            "state": "unavailable",
            "detail": format!("renderer probe failed: {:?}", reason),
        }),
        Err(error) => serde_json::json!({ "state": "unavailable", "detail": error }),
    }
}

fn remove_tree_if_exists(path: &std::path::Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }

    fs::remove_dir_all(path)
        .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    Ok(Some(path.to_string_lossy().to_string()))
}

fn request_probe(probe: RendererProbeRequest) -> Result<RendererProbeResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let timeout = renderer_probe_timeout(&probe);
    let request = RendererProbeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        probe,
    };

    let request_path = renderer_probe_bridge_request_path(&request_id);
    let response_path = renderer_probe_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create probe request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create probe response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize probe request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write probe request: {error}"))?;

    let started = SystemTime::now();
    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for probe response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<RendererProbeBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse probe response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);
                return Ok(response.result);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn require_renderer_landing() -> Result<RendererProbeSnapshot, String> {
    match request_probe(RendererProbeRequest::LandingBasics)? {
        RendererProbeResult::Success { snapshot } => Ok(snapshot),
        RendererProbeResult::Failure { reason } => {
            Err(format!("renderer landing probe failed: {:?}", reason))
        }
    }
}

fn renderer_probe_timeout(probe: &RendererProbeRequest) -> Duration {
    match probe {
        RendererProbeRequest::LandingBasics => Duration::from_secs(10),
        RendererProbeRequest::MessagingState => Duration::from_secs(10),
    }
}

fn request_inspect() -> Result<InspectResult, String> {
    request_inspect_with_timeout(Duration::from_secs(15))
}

fn request_inspect_with_timeout(timeout: Duration) -> Result<InspectResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = InspectBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    let request_path = inspect_bridge_request_path(&request_id);
    let response_path = inspect_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create inspect request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create inspect response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize inspect request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write inspect request: {error}"))?;

    let started = SystemTime::now();
    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for inspect response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<InspectBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse inspect response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);
                return Ok(response.result);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn request_controller_runtime_with_timeout(
    timeout: Duration,
) -> Result<ControllerRuntimeBridgeResponse, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = ControllerRuntimeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    let request_path = controller_runtime_bridge_request_path(&request_id);
    let response_path = controller_runtime_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create controller runtime request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create controller runtime response dir: {error}")
        })?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize controller runtime request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write controller runtime request: {error}"))?;

    let started = SystemTime::now();
    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for controller runtime response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<ControllerRuntimeBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse controller runtime response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);
                return Ok(response);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn run_cargo_owned(workdir: &std::path::Path, args: &[String]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to run cargo {:?}: {error}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {:?} exited with status {status}", args))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        command_is_renderer_dev_server, command_is_tauri_host, parse_command_line,
        reject_extra_args, renderer_probe_timeout,
    };
    use std::time::Duration;
    use stim_shared::inspection::RendererProbeRequest;

    #[test]
    fn renderer_inspect_probes_have_short_timeout_budgets() {
        assert_eq!(
            renderer_probe_timeout(&RendererProbeRequest::LandingBasics),
            Duration::from_secs(10)
        );
        assert_eq!(
            renderer_probe_timeout(&RendererProbeRequest::MessagingState),
            Duration::from_secs(10)
        );
    }

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
    fn recognizes_renderer_vite_dev_server_process() {
        let command = format!(
            "node {}/node_modules/.bin/../vite/bin/vite.js --host 127.0.0.1 --port 1420",
            stim_shared::paths::renderer_vite_dir().display()
        );

        assert!(command_is_renderer_dev_server(&command));
        assert!(!command_is_renderer_dev_server(
            "node /tmp/other/vite.js --host 127.0.0.1 --port 1420"
        ));
    }

    #[test]
    fn recognizes_tauri_host_process() {
        let command = format!(
            "{} --stim-stamp-app=tauri --stim-stamp-namespace=default",
            stim_platform::paths::workspace_root()
                .join("target")
                .join("debug")
                .join("stim-tauri")
                .display()
        );

        assert!(command_is_tauri_host(&command));
        assert!(!command_is_tauri_host("/tmp/stim-tauri"));
    }
}
