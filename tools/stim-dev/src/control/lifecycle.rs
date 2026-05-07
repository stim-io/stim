use std::{fs, time::Duration};

use stim_shared::inspection::RendererProbeRequest;

use crate::shared::{
    bridge::{request_controller_runtime, request_inspect_with_timeout, request_probe},
    clock::timestamp_now,
};

use super::{
    current_namespace,
    output::{bridge_result_json, process_list_json, renderer_probe_result_json},
    processes::{stamped_processes_for_namespace, stop_matching_processes},
};

pub(crate) fn status() -> Result<(), String> {
    let namespace = current_namespace();
    let processes = stamped_processes_for_namespace(&namespace)?;
    let host = request_inspect_with_timeout(Duration::from_secs(15));
    let controller_runtime = request_controller_runtime(Duration::from_secs(15));
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

pub(crate) fn list() -> Result<(), String> {
    let namespace = current_namespace();
    let processes = stamped_processes_for_namespace(&namespace)?;
    let host = request_inspect_with_timeout(Duration::from_secs(2));
    let controller_runtime = request_controller_runtime(Duration::from_secs(2));
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

pub(crate) fn stop() -> Result<(), String> {
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

pub(crate) fn reset() -> Result<(), String> {
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
    let criteria = stim_sidecar::process::StampedProcessCriteria {
        namespace: Some(namespace.to_string()),
        ..stim_sidecar::process::StampedProcessCriteria::default()
    };
    stop_matching_processes(&criteria)
}

fn remove_tree_if_exists(path: &std::path::Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }

    fs::remove_dir_all(path)
        .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    Ok(Some(path.to_string_lossy().to_string()))
}
