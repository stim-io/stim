use std::{fs, time::Duration};

use stim_shared::inspection::{
    InspectResult, RendererProbeRequest, RendererProbeResult, RendererProbeSnapshot,
    ScreenshotResult,
};
use stim_sidecar::{identity::namespace_or_default, process::StampedProcessCriteria};

use crate::{
    bridge::{
        request_controller_runtime_with_timeout, request_inspect, request_inspect_with_timeout,
        request_probe, request_screenshot,
    },
    clock::timestamp_now,
};

pub(crate) fn current_namespace() -> String {
    namespace_or_default(
        std::env::var(stim_sidecar::identity::SIDECAR_NAMESPACE_ENV)
            .ok()
            .as_deref(),
    )
}

pub(crate) fn inspect(args: Vec<String>) -> Result<(), String> {
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
    match request_screenshot(label)? {
        ScreenshotResult::Success { path, .. } => {
            println!("{path}");
            Ok(())
        }
        ScreenshotResult::Failure { reason } => Err(format!("screenshot failed: {:?}", reason)),
    }
}

pub(crate) fn status() -> Result<(), String> {
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

pub(crate) fn list() -> Result<(), String> {
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

pub(crate) fn stop_matching_processes(
    criteria: &StampedProcessCriteria,
) -> Result<stim_platform::process::StopProcessResult, String> {
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let matched = stim_sidecar::process::matching_stamped_processes(&processes, criteria);
    let root_pids = matched
        .iter()
        .map(|process| process.pid)
        .collect::<Vec<_>>();
    let tree_pids = stim_platform::process::collect_process_tree_pids(&processes, &root_pids);

    stim_platform::process::stop_processes(&tree_pids)
        .map_err(|error| format!("failed to stop stamped processes: {error}"))
}

pub(crate) fn stop_renderer_dev_server_processes(
) -> Result<stim_platform::process::StopProcessResult, String> {
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

pub(crate) fn stop_tauri_host_processes(
) -> Result<stim_platform::process::StopProcessResult, String> {
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

pub(crate) fn stamped_processes_for_namespace(
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

pub(crate) fn require_renderer_landing() -> Result<RendererProbeSnapshot, String> {
    match request_probe(RendererProbeRequest::LandingBasics)? {
        RendererProbeResult::Success { snapshot } => Ok(snapshot),
        RendererProbeResult::Failure { reason } => {
            Err(format!("renderer landing probe failed: {:?}", reason))
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{command_is_renderer_dev_server, command_is_tauri_host};

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
