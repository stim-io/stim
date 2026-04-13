use std::{fs, thread, time::Duration};

use tauri::{AppHandle, Manager, Runtime};

use stim_shared::{
    inspection::{
        InspectBridgeRequest, InspectBridgeResponse, ScreenshotBridgeRequest,
        ScreenshotBridgeResponse,
    },
    paths::{
        inspect_bridge_requests_dir, inspect_bridge_response_path, inspect_bridge_responses_dir,
        screenshot_bridge_requests_dir, screenshot_bridge_response_path,
        screenshot_bridge_responses_dir,
    },
};

use crate::inspection::inspect::inspect_main_window;
use crate::inspection::renderer_probe::poll_renderer_probe_requests;
use crate::inspection::screenshot::capture_main_window_screenshot;

pub fn start_inspection_bridge<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || loop {
        if let Err(error) = poll_screenshot_requests(&app) {
            eprintln!("[stim-tauri][inspection] screenshot bridge poll failed: {error}");
        }

        if let Err(error) = poll_inspect_requests(&app) {
            eprintln!("[stim-tauri][inspection] inspect bridge poll failed: {error}");
        }

        if let Err(error) = poll_renderer_probe_requests(&app) {
            eprintln!("[stim-tauri][inspection] renderer probe bridge poll failed: {error}");
        }

        thread::sleep(Duration::from_millis(500));
    });
}

fn poll_screenshot_requests<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    fs::create_dir_all(screenshot_bridge_requests_dir())
        .map_err(|error| format!("failed to create screenshot request dir: {error}"))?;
    fs::create_dir_all(screenshot_bridge_responses_dir())
        .map_err(|error| format!("failed to create screenshot response dir: {error}"))?;

    let mut entries = fs::read_dir(screenshot_bridge_requests_dir())
        .map_err(|error| format!("failed to read screenshot request dir: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let request_path = entry.path();
        let request_content = fs::read_to_string(&request_path)
            .map_err(|error| format!("failed to read screenshot request: {error}"))?;
        let request = match serde_json::from_str::<ScreenshotBridgeRequest>(&request_content) {
            Ok(request) => request,
            Err(_) => {
                let _ = fs::remove_file(&request_path);
                continue;
            }
        };

        let window = app.get_webview_window("main");
        let result = capture_main_window_screenshot(window.as_ref(), request.label.as_deref());
        let response = ScreenshotBridgeResponse {
            request_id: request.request_id.clone(),
            requested_at: request.requested_at.clone(),
            responded_at: crate::inspection::screenshot::timestamp_now(),
            result,
        };

        let response_path = screenshot_bridge_response_path(&request.request_id);
        let response_body = serde_json::to_string_pretty(&response)
            .map_err(|error| format!("failed to serialize screenshot response: {error}"))?;
        fs::write(&response_path, format!("{response_body}\n"))
            .map_err(|error| format!("failed to write screenshot response: {error}"))?;
        let _ = fs::remove_file(&request_path);
    }

    Ok(())
}

fn poll_inspect_requests<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    fs::create_dir_all(inspect_bridge_requests_dir())
        .map_err(|error| format!("failed to create inspect request dir: {error}"))?;
    fs::create_dir_all(inspect_bridge_responses_dir())
        .map_err(|error| format!("failed to create inspect response dir: {error}"))?;

    let mut entries = fs::read_dir(inspect_bridge_requests_dir())
        .map_err(|error| format!("failed to read inspect request dir: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let request_path = entry.path();
        let request_content = fs::read_to_string(&request_path)
            .map_err(|error| format!("failed to read inspect request: {error}"))?;
        let request = match serde_json::from_str::<InspectBridgeRequest>(&request_content) {
            Ok(request) => request,
            Err(_) => {
                let _ = fs::remove_file(&request_path);
                continue;
            }
        };

        let result = inspect_main_window(app);
        let response = InspectBridgeResponse {
            request_id: request.request_id.clone(),
            requested_at: request.requested_at.clone(),
            responded_at: crate::inspection::screenshot::timestamp_now(),
            result,
        };

        let response_path = inspect_bridge_response_path(&request.request_id);
        let response_body = serde_json::to_string_pretty(&response)
            .map_err(|error| format!("failed to serialize inspect response: {error}"))?;
        fs::write(&response_path, format!("{response_body}\n"))
            .map_err(|error| format!("failed to write inspect response: {error}"))?;
        let _ = fs::remove_file(&request_path);
    }

    Ok(())
}
