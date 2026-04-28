use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use stim_shared::{
    control_plane::{
        ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot, ControllerRuntimeState,
        SIDECAR_NAMESPACE_ENV,
    },
    paths::workspace_root,
};
use stim_sidecar::{
    identity::{
        mode_or_default, namespace_or_default, SidecarMode, SidecarStamp, SIDECAR_MODE_ENV,
        SOURCE_APP_TAURI,
    },
    ready::wait_for_ready_line,
    stamp::create_stamp_args,
};
use tauri::Manager;

const CONTROLLER_BIN_ENV: &str = "STIM_CONTROLLER_BIN";
const CONTROLLER_ENDPOINT_ENV: &str = "STIM_CONTROLLER_ENDPOINT";
const CONTROLLER_INSTANCE_ENV: &str = "STIM_CONTROLLER_INSTANCE_ID";
const CONTROLLER_READY_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ControllerRuntimeManager(pub Mutex<ControllerRuntimeHandle>);

pub struct ControllerRuntimeHandle {
    process: ControllerRuntimeProcess,
    detail: Option<String>,
    heartbeat_sequence: u64,
    http_base_url: Option<String>,
    stamp: SidecarStamp,
    instance_id: String,
    ready_at: String,
}

enum ControllerRuntimeProcess {
    Attached,
    Owned(Child),
}

pub fn start_controller_runtime<R: tauri::Runtime>(app: &mut tauri::App<R>) {
    let namespace = namespace_or_default(std::env::var(SIDECAR_NAMESPACE_ENV).ok().as_deref());
    let handle = spawn_controller_sidecar(&namespace)
        .expect("failed to start local stim controller sidecar runtime");

    app.manage(ControllerRuntimeManager(Mutex::new(handle)));
}

fn spawn_controller_sidecar(namespace: &str) -> Result<ControllerRuntimeHandle, String> {
    let mode = mode_or_default(
        std::env::var(SIDECAR_MODE_ENV).ok().as_deref(),
        SidecarMode::Dev,
    );

    if let Ok(endpoint) = std::env::var(CONTROLLER_ENDPOINT_ENV) {
        let instance_id = std::env::var(CONTROLLER_INSTANCE_ENV)
            .unwrap_or_else(|_| format!("attached-controller-{}", timestamp_now()));
        return Ok(ControllerRuntimeHandle {
            process: ControllerRuntimeProcess::Attached,
            detail: Some(format!("controller attached via {CONTROLLER_ENDPOINT_ENV}")),
            heartbeat_sequence: 0,
            http_base_url: Some(endpoint),
            stamp: SidecarStamp {
                app: "controller".into(),
                namespace: namespace.to_string(),
                mode,
                source: SOURCE_APP_TAURI.into(),
            },
            instance_id,
            ready_at: timestamp_now(),
        });
    }

    let stamp = SidecarStamp {
        app: "controller".into(),
        namespace: namespace.to_string(),
        mode,
        source: SOURCE_APP_TAURI.into(),
    };
    let stamp_args = create_stamp_args(&stamp);
    let mut command = controller_command(&stamp_args);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn controller sidecar: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "controller sidecar stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, CONTROLLER_READY_TIMEOUT)
        .map_err(|error| format!("controller sidecar ready failed: {error}"))?;

    if !ready.is_ready_line() {
        return Err("controller sidecar emitted an unexpected ready line".into());
    }

    if ready.stamp != stamp {
        return Err("controller sidecar ready stamp did not match launch stamp".into());
    }

    if ready.role != "controller-runtime" {
        return Err(format!(
            "controller sidecar ready role did not match expected controller-runtime: {}",
            ready.role
        ));
    }

    Ok(ControllerRuntimeHandle {
        process: ControllerRuntimeProcess::Owned(child),
        detail: Some("controller sidecar launched by tauri host".into()),
        heartbeat_sequence: 0,
        http_base_url: ready.endpoint,
        stamp,
        instance_id: ready.instance_id,
        ready_at: ready.ready_at,
    })
}

fn controller_command(stamp_args: &[String]) -> Command {
    if let Ok(binary) = std::env::var(CONTROLLER_BIN_ENV) {
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

impl ControllerRuntimeHandle {
    fn snapshot(&mut self) -> ControllerRuntimeSnapshot {
        let (state, detail) = self.current_state_and_detail();

        ControllerRuntimeSnapshot {
            namespace: self.stamp.namespace.clone(),
            instance_id: self.instance_id.clone(),
            published_at: timestamp_now(),
            state,
            http_base_url: self.http_base_url.clone(),
            detail: detail.or_else(|| Some(format!("controller ready at {}", self.ready_at))),
        }
    }

    fn heartbeat(&mut self) -> ControllerRuntimeHeartbeat {
        self.heartbeat_sequence += 1;
        let (state, _) = self.current_state_and_detail();

        ControllerRuntimeHeartbeat {
            namespace: self.stamp.namespace.clone(),
            instance_id: self.instance_id.clone(),
            published_at: timestamp_now(),
            sequence: self.heartbeat_sequence,
            state,
        }
    }

    fn current_state_and_detail(&mut self) -> (ControllerRuntimeState, Option<String>) {
        match &mut self.process {
            ControllerRuntimeProcess::Attached => {
                (ControllerRuntimeState::Ready, self.detail.clone())
            }
            ControllerRuntimeProcess::Owned(child) => match child.try_wait() {
                Ok(Some(status)) => (
                    ControllerRuntimeState::Stopped,
                    Some(format!("controller sidecar exited with status {status}")),
                ),
                Ok(None) => (ControllerRuntimeState::Ready, self.detail.clone()),
                Err(error) => (
                    ControllerRuntimeState::Degraded,
                    Some(format!("failed to inspect controller sidecar: {error}")),
                ),
            },
        }
    }
}

impl Drop for ControllerRuntimeHandle {
    fn drop(&mut self) {
        if let ControllerRuntimeProcess::Owned(child) = &mut self.process {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

pub fn controller_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> ControllerRuntimeSnapshot {
    app.state::<ControllerRuntimeManager>()
        .0
        .lock()
        .expect("controller runtime state poisoned")
        .snapshot()
}

pub fn controller_heartbeat<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> ControllerRuntimeHeartbeat {
    app.state::<ControllerRuntimeManager>()
        .0
        .lock()
        .expect("controller runtime state poisoned")
        .heartbeat()
}

#[tauri::command]
pub fn controller_runtime_snapshot(app: tauri::AppHandle) -> ControllerRuntimeSnapshot {
    controller_snapshot(&app)
}

#[tauri::command]
pub fn controller_runtime_heartbeat(app: tauri::AppHandle) -> ControllerRuntimeHeartbeat {
    controller_heartbeat(&app)
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}
