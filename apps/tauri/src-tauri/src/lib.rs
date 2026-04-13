mod inspection;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            inspection::renderer_probe::manage_renderer_probe_state(app);
            inspection::renderer_probe::register_renderer_probe_listener(app.handle());
            inspection::request_handler::start_inspection_bridge(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run stim tauri shell");
}
