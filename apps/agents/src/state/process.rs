use std::{
    fs::{self, OpenOptions},
    process::{Child, Command, Stdio},
};

use super::{config::SantiLaunchConfig, model::AgentInstanceConfig, AgentRegistryError};

pub(super) fn spawn_santi_instance(
    instance: &AgentInstanceConfig,
    launch: &SantiLaunchConfig,
) -> Result<Child, AgentRegistryError> {
    let mut command = Command::new(&launch.command);
    let log = open_launch_log(instance)?;
    let stderr = log
        .try_clone()
        .map_err(|error| AgentRegistryError::LaunchFailed(error.to_string()))?;
    command
        .args(&launch.args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(stderr));

    if let Some(cwd) = launch.cwd.as_ref().filter(|cwd| !cwd.trim().is_empty()) {
        command.current_dir(cwd);
    }

    for (key, value) in &launch.env {
        command.env(key, value);
    }

    detach_process_group(&mut command);

    command
        .spawn()
        .map_err(|error| AgentRegistryError::LaunchFailed(error.to_string()))
}

pub(super) fn stop_launched_process(
    pid: u32,
) -> Result<stim_platform::process::StopProcessResult, AgentRegistryError> {
    let processes = stim_platform::process::list_process_snapshots()
        .map_err(|error| AgentRegistryError::StopFailed(error.to_string()))?;
    let mut pids = stim_platform::process::collect_process_tree_pids(&processes, &[pid]);
    if pids.is_empty() {
        pids.push(pid);
    }

    stim_platform::process::stop_processes(&pids)
        .map_err(|error| AgentRegistryError::StopFailed(error.to_string()))
}

fn open_launch_log(instance: &AgentInstanceConfig) -> Result<std::fs::File, AgentRegistryError> {
    let layout = stim_sidecar::layout::SidecarLayout::new(
        stim_platform::paths::dev_root(),
        Some(&instance.namespace),
    );
    let log_name = format!("santi-{}", safe_log_name(&instance.id));
    let log_path = layout.app_log_path(&log_name);
    let parent = log_path.parent().ok_or_else(|| {
        AgentRegistryError::LaunchFailed(format!(
            "managed Santi log path has no parent: {}",
            log_path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        AgentRegistryError::LaunchFailed(format!("failed to create {}: {error}", parent.display()))
    })?;

    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| {
            AgentRegistryError::LaunchFailed(format!(
                "failed to open {}: {error}",
                log_path.display()
            ))
        })
}

fn safe_log_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn detach_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(not(unix))]
    {
        let _ = command;
    }
}
