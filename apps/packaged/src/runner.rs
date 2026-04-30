use std::{
    env,
    process::{Command, Stdio},
    time::Duration,
};

use stim_shared::paths::workspace_root;
use stim_sidecar::{
    identity::{SIDECAR_MODE_ENV, SIDECAR_NAMESPACE_ENV},
    ready::{wait_for_ready_line, SidecarReadyLine},
    stamp::{create_stamp_args, read_stamp},
};

use crate::{clock::timestamp_now, plan::PackagedSidecarEntry};

const CONTROLLER_BIN_ENV: &str = "STIM_CONTROLLER_BIN";
pub(crate) const CONTROLLER_ENDPOINT_ENV: &str = "STIM_CONTROLLER_ENDPOINT";
pub(crate) const CONTROLLER_INSTANCE_ENV: &str = "STIM_CONTROLLER_INSTANCE_ID";
const CONTROLLER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const RUNNER_READY_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) fn spawn_controller_ready(
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

pub(crate) fn spawn_runner_ready(
    command_name: &str,
    sidecar: &PackagedSidecarEntry,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_runner_ready_with_env(command_name, sidecar, &[])
}

pub(crate) fn spawn_runner_ready_with_env(
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

pub(crate) fn run_renderer_sidecar(args: Vec<String>) -> Result<(), String> {
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

pub(crate) fn run_tauri_sidecar(args: Vec<String>) -> Result<(), String> {
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
