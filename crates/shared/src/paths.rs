use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve stim workspace root")
}

pub fn renderer_app_dir() -> PathBuf {
    workspace_root().join("apps/renderer")
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

pub fn inspection_root() -> PathBuf {
    dev_root().join("inspection")
}

pub fn screenshot_bridge_requests_dir() -> PathBuf {
    inspection_root().join("screenshot-bridge/requests")
}

pub fn screenshot_bridge_responses_dir() -> PathBuf {
    inspection_root().join("screenshot-bridge/responses")
}

pub fn main_window_screenshots_dir() -> PathBuf {
    inspection_root().join("main-window-screenshots")
}

pub fn screenshot_bridge_request_path(request_id: &str) -> PathBuf {
    screenshot_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn screenshot_bridge_response_path(request_id: &str) -> PathBuf {
    screenshot_bridge_responses_dir().join(format!("{request_id}.json"))
}

pub fn inspect_bridge_requests_dir() -> PathBuf {
    inspection_root().join("inspect-bridge/requests")
}

pub fn inspect_bridge_responses_dir() -> PathBuf {
    inspection_root().join("inspect-bridge/responses")
}

pub fn inspect_bridge_request_path(request_id: &str) -> PathBuf {
    inspect_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn inspect_bridge_response_path(request_id: &str) -> PathBuf {
    inspect_bridge_responses_dir().join(format!("{request_id}.json"))
}

pub fn renderer_probe_bridge_requests_dir() -> PathBuf {
    inspection_root().join("renderer-probe-bridge/requests")
}

pub fn renderer_probe_bridge_responses_dir() -> PathBuf {
    inspection_root().join("renderer-probe-bridge/responses")
}

pub fn renderer_probe_bridge_request_path(request_id: &str) -> PathBuf {
    renderer_probe_bridge_requests_dir().join(format!("{request_id}.json"))
}

pub fn renderer_probe_bridge_response_path(request_id: &str) -> PathBuf {
    renderer_probe_bridge_responses_dir().join(format!("{request_id}.json"))
}
