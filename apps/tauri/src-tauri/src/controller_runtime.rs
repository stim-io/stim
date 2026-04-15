use std::sync::Mutex;

use stim_controller::service::{spawn_local_controller, ControllerServiceHandle};
use stim_shared::control_plane::{
    ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot, IPC_NAMESPACE_ENV,
};
use tauri::Manager;

pub struct ControllerRuntimeState(pub Mutex<ControllerServiceHandle>);

pub fn start_controller_runtime<R: tauri::Runtime>(app: &mut tauri::App<R>) {
    let namespace = std::env::var(IPC_NAMESPACE_ENV).ok();
    let handle = spawn_local_controller(namespace.as_deref())
        .expect("failed to start local stim controller runtime");
    app.manage(ControllerRuntimeState(Mutex::new(handle)));
}

pub fn controller_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> ControllerRuntimeSnapshot {
    app.state::<ControllerRuntimeState>()
        .0
        .lock()
        .expect("controller runtime state poisoned")
        .snapshot()
}

pub fn controller_heartbeat<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> ControllerRuntimeHeartbeat {
    app.state::<ControllerRuntimeState>()
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
