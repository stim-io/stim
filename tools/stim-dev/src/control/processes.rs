use stim_sidecar::process::StampedProcessCriteria;

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

pub(crate) fn stop_renderer_processes() -> Result<stim_platform::process::StopProcessResult, String>
{
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| format!("failed to list processes: {error}"))?;
    let matched_pids = processes
        .iter()
        .filter(|process| is_renderer_dev_server(&process.command))
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

pub(crate) fn command_is_tauri_host(command: &str) -> bool {
    let tauri_binary = stim_platform::paths::workspace_root()
        .join("target")
        .join("debug")
        .join("stim-tauri");
    command.contains(tauri_binary.to_string_lossy().as_ref())
}

pub(crate) fn is_renderer_dev_server(command: &str) -> bool {
    let renderer_vite_dir = stim_shared::paths::renderer_vite_dir();
    let renderer_vite_dir = renderer_vite_dir.to_string_lossy();

    command.contains(renderer_vite_dir.as_ref())
        && command.contains("vite")
        && command.contains("--host 127.0.0.1")
        && command.contains("--port 1420")
}
