mod controller_runtime;
mod inspection;

const DEFAULT_RENDERER_URL: &str = "http://127.0.0.1:1420";

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            controller_runtime::controller_runtime_snapshot,
            controller_runtime::controller_runtime_heartbeat
        ])
        .setup(|app| {
            create_main_window(app)?;
            controller_runtime::start_controller_runtime(app);
            inspection::renderer_probe::manage_renderer_probe_state(app);
            inspection::renderer_probe::register_renderer_probe_listener(app.handle());
            inspection::request_handler::start_inspection_bridge(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run stim tauri shell");
}

fn create_main_window<R: tauri::Runtime>(app: &mut tauri::App<R>) -> tauri::Result<()> {
    let renderer_url =
        renderer_url_from_launch_bridge().unwrap_or_else(|| DEFAULT_RENDERER_URL.into());
    let url = tauri::Url::parse(&renderer_url).map_err(tauri::Error::InvalidUrl)?;

    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
        .title("stim")
        .inner_size(1200.0, 820.0)
        .resizable(true)
        .build()?;

    Ok(())
}

fn renderer_url_from_launch_bridge() -> Option<String> {
    let namespace = std::env::var(stim_sidecar::identity::SIDECAR_NAMESPACE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| stim_sidecar::identity::DEFAULT_NAMESPACE.to_string());
    let sidecar_mode = std::env::var(stim_sidecar::identity::SIDECAR_MODE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            stim_sidecar::identity::SidecarMode::Dev
                .as_str()
                .to_string()
        });
    let path = stim_shared::paths::renderer_delivery_launch_bridge_path(&sidecar_mode, &namespace);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let bridge = match serde_json::from_str::<
        stim_shared::control_plane::RendererDeliveryLaunchBridge,
    >(&content)
    {
        Ok(bridge) => bridge,
        Err(_) => return Some(String::new()),
    };

    if bridge.namespace == namespace {
        Some(bridge.renderer_url)
    } else {
        Some(String::new())
    }
}
