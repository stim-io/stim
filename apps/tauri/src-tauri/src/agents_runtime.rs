use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use stim_shared::{
    control_plane::{
        AgentsRuntimeHeartbeat, AgentsRuntimeSnapshot, AgentsRuntimeState, SIDECAR_NAMESPACE_ENV,
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

const AGENTS_BIN_ENV: &str = "STIM_AGENTS_BIN";
const AGENTS_ENDPOINT_ENV: &str = "STIM_AGENTS_ENDPOINT";
const AGENTS_INSTANCE_ENV: &str = "STIM_AGENTS_INSTANCE_ID";
const AGENTS_READY_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AgentsRuntimeManager(pub Mutex<AgentsRuntimeHandle>);

pub struct AgentsRuntimeHandle {
    process: AgentsRuntimeProcess,
    detail: Option<String>,
    heartbeat_sequence: u64,
    http_base_url: Option<String>,
    stamp: SidecarStamp,
    instance_id: String,
    ready_at: String,
}

enum AgentsRuntimeProcess {
    Attached,
    Owned(Child),
}

pub fn start_agents_runtime<R: tauri::Runtime>(app: &mut tauri::App<R>) {
    let namespace = namespace_or_default(std::env::var(SIDECAR_NAMESPACE_ENV).ok().as_deref());
    let handle = spawn_agents_sidecar(&namespace)
        .expect("failed to start local stim agents sidecar runtime");

    app.manage(AgentsRuntimeManager(Mutex::new(handle)));
}

fn spawn_agents_sidecar(namespace: &str) -> Result<AgentsRuntimeHandle, String> {
    let mode = mode_or_default(
        std::env::var(SIDECAR_MODE_ENV).ok().as_deref(),
        SidecarMode::Dev,
    );

    if let Ok(endpoint) = std::env::var(AGENTS_ENDPOINT_ENV) {
        let instance_id = std::env::var(AGENTS_INSTANCE_ENV)
            .unwrap_or_else(|_| format!("attached-agents-{}", timestamp_now()));
        return Ok(AgentsRuntimeHandle {
            process: AgentsRuntimeProcess::Attached,
            detail: Some(format!("agents attached via {AGENTS_ENDPOINT_ENV}")),
            heartbeat_sequence: 0,
            http_base_url: Some(endpoint),
            stamp: SidecarStamp {
                app: "agents".into(),
                namespace: namespace.to_string(),
                mode,
                source: SOURCE_APP_TAURI.into(),
            },
            instance_id,
            ready_at: timestamp_now(),
        });
    }

    let stamp = SidecarStamp {
        app: "agents".into(),
        namespace: namespace.to_string(),
        mode,
        source: SOURCE_APP_TAURI.into(),
    };
    let stamp_args = create_stamp_args(&stamp);
    let mut command = agents_command(&stamp_args);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn agents sidecar: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "agents sidecar stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, AGENTS_READY_TIMEOUT)
        .map_err(|error| format!("agents sidecar ready failed: {error}"))?;

    if !ready.is_ready_line() {
        return Err("agents sidecar emitted an unexpected ready line".into());
    }

    if ready.stamp != stamp {
        return Err("agents sidecar ready stamp did not match launch stamp".into());
    }

    if ready.role != "agents-runtime" {
        return Err(format!(
            "agents sidecar ready role did not match expected agents-runtime: {}",
            ready.role
        ));
    }

    Ok(AgentsRuntimeHandle {
        process: AgentsRuntimeProcess::Owned(child),
        detail: Some("agents sidecar launched by tauri host".into()),
        heartbeat_sequence: 0,
        http_base_url: ready.endpoint,
        stamp,
        instance_id: ready.instance_id,
        ready_at: ready.ready_at,
    })
}

fn agents_command(stamp_args: &[String]) -> Command {
    if let Ok(binary) = std::env::var(AGENTS_BIN_ENV) {
        let mut command = Command::new(binary);

        command.arg("serve").args(stamp_args);
        return command;
    }

    let mut command = Command::new("cargo");

    command
        .args(["run", "-p", "stim-agents", "--", "serve"])
        .args(stamp_args)
        .current_dir(workspace_root());

    command
}

impl AgentsRuntimeHandle {
    fn snapshot(&mut self) -> AgentsRuntimeSnapshot {
        let (state, detail) = self.current_state_and_detail();

        AgentsRuntimeSnapshot {
            namespace: self.stamp.namespace.clone(),
            instance_id: self.instance_id.clone(),
            published_at: timestamp_now(),
            state,
            http_base_url: self.http_base_url.clone(),
            detail: detail.or_else(|| Some(format!("agents ready at {}", self.ready_at))),
        }
    }

    fn heartbeat(&mut self) -> AgentsRuntimeHeartbeat {
        self.heartbeat_sequence += 1;
        let (state, _) = self.current_state_and_detail();

        AgentsRuntimeHeartbeat {
            namespace: self.stamp.namespace.clone(),
            instance_id: self.instance_id.clone(),
            published_at: timestamp_now(),
            sequence: self.heartbeat_sequence,
            state,
        }
    }

    fn current_state_and_detail(&mut self) -> (AgentsRuntimeState, Option<String>) {
        match &mut self.process {
            AgentsRuntimeProcess::Attached => (AgentsRuntimeState::Ready, self.detail.clone()),
            AgentsRuntimeProcess::Owned(child) => match child.try_wait() {
                Ok(Some(status)) => (
                    AgentsRuntimeState::Stopped,
                    Some(format!("agents sidecar exited with status {status}")),
                ),
                Ok(None) => (AgentsRuntimeState::Ready, self.detail.clone()),
                Err(error) => (
                    AgentsRuntimeState::Degraded,
                    Some(format!("failed to inspect agents sidecar: {error}")),
                ),
            },
        }
    }
}

impl Drop for AgentsRuntimeHandle {
    fn drop(&mut self) {
        if let AgentsRuntimeProcess::Owned(child) = &mut self.process {
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

pub fn agents_snapshot<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AgentsRuntimeSnapshot {
    app.state::<AgentsRuntimeManager>()
        .0
        .lock()
        .expect("agents runtime state poisoned")
        .snapshot()
}

pub fn agents_heartbeat<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AgentsRuntimeHeartbeat {
    app.state::<AgentsRuntimeManager>()
        .0
        .lock()
        .expect("agents runtime state poisoned")
        .heartbeat()
}

#[tauri::command]
pub fn agents_runtime_snapshot(app: tauri::AppHandle) -> AgentsRuntimeSnapshot {
    agents_snapshot(&app)
}

#[tauri::command]
pub fn agents_runtime_heartbeat(app: tauri::AppHandle) -> AgentsRuntimeHeartbeat {
    agents_heartbeat(&app)
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}
