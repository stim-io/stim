use std::time::Duration;

use stim_sidecar::{identity::SidecarMode, process::StampedProcessCriteria};

use crate::{
    bridge::{request_controller_runtime_with_timeout, request_inspect_with_timeout},
    cli::{StartOptions, StartTarget},
    runtime_control::{
        current_namespace, require_renderer_landing, stamped_processes_for_namespace,
        stop_matching_processes, stop_renderer_dev_server_processes, stop_tauri_host_processes,
    },
    sidecars::{
        spawn_controller_ready, spawn_controller_ready_detached, spawn_renderer_ready,
        spawn_renderer_ready_detached, spawn_tauri, spawn_tauri_detached, start_controller,
        start_renderer_foreground, start_tauri, wait_children, write_renderer_delivery_bridge,
    },
};

pub(crate) enum ExistingInstancePolicy {
    Reject,
    Allow,
}

pub(crate) fn start(
    options: StartOptions,
    existing_policy: ExistingInstancePolicy,
) -> Result<(), String> {
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

pub(crate) fn restart(options: StartOptions) -> Result<(), String> {
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
        stim_sidecar::identity::SOURCE_TOOL_STIM_DEV,
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
        stim_sidecar::identity::SOURCE_TOOL_STIM_DEV,
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
        stim_sidecar::identity::SOURCE_TOOL_STIM_DEV,
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
        stim_sidecar::identity::SOURCE_TOOL_STIM_DEV,
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
