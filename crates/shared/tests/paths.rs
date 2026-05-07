use std::sync::{Mutex, OnceLock};

use stim_shared::{
    control_plane::{LEGACY_IPC_NAMESPACE_ENV, SIDECAR_NAMESPACE_ENV},
    paths::{
        inspect_bridge_request_path, main_window_screenshots_dir, renderer_probe_response_path,
    },
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn bridge_paths_use_namespace() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var(SIDECAR_NAMESPACE_ENV, "dev-a");
    std::env::remove_var(LEGACY_IPC_NAMESPACE_ENV);

    assert!(inspect_bridge_request_path("req-1")
        .to_string_lossy()
        .contains(".tmp/dev/dev-a/bridges/inspect/requests/req-1.json"));
    assert!(main_window_screenshots_dir()
        .to_string_lossy()
        .contains(".tmp/dev/dev-a/bridges/screenshot/artifacts/main-window"));

    std::env::remove_var(SIDECAR_NAMESPACE_ENV);
}

#[test]
fn legacy_namespace_fallback() {
    let _guard = env_lock().lock().unwrap();
    std::env::remove_var(SIDECAR_NAMESPACE_ENV);
    std::env::set_var(LEGACY_IPC_NAMESPACE_ENV, "legacy-a");

    assert!(renderer_probe_response_path("req-2")
        .to_string_lossy()
        .contains(".tmp/dev/legacy-a/bridges/renderer-probe/responses/req-2.json"));

    std::env::remove_var(LEGACY_IPC_NAMESPACE_ENV);
}

fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}
