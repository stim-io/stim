use std::path::PathBuf;

use crate::control_plane::{LEGACY_IPC_NAMESPACE_ENV, SIDECAR_NAMESPACE_ENV};

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve stim workspace root")
}

pub fn renderer_app_dir() -> PathBuf {
    workspace_root().join("apps/renderer")
}

pub fn renderer_vite_dir() -> PathBuf {
    renderer_app_dir().join("vite")
}

pub fn tauri_app_dir() -> PathBuf {
    workspace_root().join("apps/tauri")
}

pub fn controller_app_dir() -> PathBuf {
    workspace_root().join("apps/controller")
}

pub fn dev_root() -> PathBuf {
    workspace_root().join(".tmp/dev")
}

pub fn sidecars_root() -> PathBuf {
    workspace_root().join(".tmp/sidecars")
}

pub fn current_namespace() -> String {
    std::env::var(SIDECAR_NAMESPACE_ENV)
        .or_else(|_| std::env::var(LEGACY_IPC_NAMESPACE_ENV))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

pub fn namespace_root() -> PathBuf {
    dev_root().join(current_namespace())
}

pub fn bridges_root() -> PathBuf {
    namespace_root().join("bridges")
}

pub fn launcher_bridge_root(sidecar_mode: &str, namespace: &str) -> PathBuf {
    sidecars_root()
        .join(sidecar_mode.trim())
        .join(namespace.trim())
        .join("bridges")
}

pub fn renderer_delivery_launch_bridge_path(sidecar_mode: &str, namespace: &str) -> PathBuf {
    launcher_bridge_root(sidecar_mode, namespace).join("renderer-delivery/launch.json")
}

pub fn screenshot_bridge_requests_dir() -> PathBuf {
    bridges_root().join("screenshot/requests")
}

pub fn screenshot_bridge_responses_dir() -> PathBuf {
    bridges_root().join("screenshot/responses")
}

pub fn main_window_screenshots_dir() -> PathBuf {
    bridges_root().join("screenshot/artifacts/main-window")
}

pub fn screenshot_bridge_request_path(request_id: &str) -> PathBuf {
    screenshot_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn screenshot_bridge_response_path(request_id: &str) -> PathBuf {
    screenshot_bridge_responses_dir().join(format!("{request_id}.json"))
}

pub fn inspect_bridge_requests_dir() -> PathBuf {
    bridges_root().join("inspect/requests")
}

pub fn inspect_bridge_responses_dir() -> PathBuf {
    bridges_root().join("inspect/responses")
}

pub fn inspect_bridge_request_path(request_id: &str) -> PathBuf {
    inspect_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn inspect_bridge_response_path(request_id: &str) -> PathBuf {
    inspect_bridge_responses_dir().join(format!("{request_id}.json"))
}

pub fn renderer_probe_bridge_requests_dir() -> PathBuf {
    bridges_root().join("renderer-probe/requests")
}

pub fn renderer_probe_bridge_responses_dir() -> PathBuf {
    bridges_root().join("renderer-probe/responses")
}

pub fn renderer_probe_bridge_request_path(request_id: &str) -> PathBuf {
    renderer_probe_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn renderer_probe_bridge_response_path(request_id: &str) -> PathBuf {
    renderer_probe_bridge_responses_dir().join(format!("{request_id}.json"))
}

pub fn controller_runtime_bridge_requests_dir() -> PathBuf {
    bridges_root().join("controller-runtime/requests")
}

pub fn controller_runtime_bridge_responses_dir() -> PathBuf {
    bridges_root().join("controller-runtime/responses")
}

pub fn controller_runtime_bridge_request_path(request_id: &str) -> PathBuf {
    controller_runtime_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn controller_runtime_bridge_response_path(request_id: &str) -> PathBuf {
    controller_runtime_bridge_responses_dir().join(format!("{request_id}.json"))
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use crate::control_plane::{LEGACY_IPC_NAMESPACE_ENV, SIDECAR_NAMESPACE_ENV};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn bridge_paths_are_namespace_scoped() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var(SIDECAR_NAMESPACE_ENV, "dev-a");
        std::env::remove_var(LEGACY_IPC_NAMESPACE_ENV);

        assert!(super::inspect_bridge_request_path("req-1")
            .to_string_lossy()
            .contains(".tmp/dev/dev-a/bridges/inspect/requests/req-1.json"));
        assert!(super::main_window_screenshots_dir()
            .to_string_lossy()
            .contains(".tmp/dev/dev-a/bridges/screenshot/artifacts/main-window"));

        std::env::remove_var(SIDECAR_NAMESPACE_ENV);
    }

    #[test]
    fn legacy_ipc_namespace_env_is_only_a_fallback() {
        let _guard = env_lock().lock().unwrap();
        std::env::remove_var(SIDECAR_NAMESPACE_ENV);
        std::env::set_var(LEGACY_IPC_NAMESPACE_ENV, "legacy-a");

        assert!(super::renderer_probe_bridge_response_path("req-2")
            .to_string_lossy()
            .contains(".tmp/dev/legacy-a/bridges/renderer-probe/responses/req-2.json"));

        std::env::remove_var(LEGACY_IPC_NAMESPACE_ENV);
    }
}
