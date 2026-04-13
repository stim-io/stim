use std::{collections::HashMap, sync::Mutex, thread, time::Duration};

use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};

use stim_shared::inspection::{
    RendererProbeBridgeRequest, RendererProbeBridgeResponse, RendererProbeEventRequest,
    RendererProbeEventResponse, RendererProbeFailureReason, RendererProbeResult,
    RENDERER_PROBE_REQUEST_EVENT, RENDERER_PROBE_RESPONSE_EVENT,
};
use stim_shared::paths::{
    renderer_probe_bridge_requests_dir, renderer_probe_bridge_response_path,
    renderer_probe_bridge_responses_dir,
};

pub struct RendererProbeResponses(pub Mutex<HashMap<String, RendererProbeEventResponse>>);

impl Default for RendererProbeResponses {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

pub fn manage_renderer_probe_state<R: Runtime>(app: &mut tauri::App<R>) {
    app.manage(RendererProbeResponses::default());
}

pub fn register_renderer_probe_listener<R: Runtime>(app: &AppHandle<R>) {
    let app_handle = app.clone();
    app.listen_any(RENDERER_PROBE_RESPONSE_EVENT, move |event| {
        let Ok(response) = serde_json::from_str::<RendererProbeEventResponse>(event.payload())
        else {
            return;
        };

        if let Ok(mut responses) = app_handle.state::<RendererProbeResponses>().0.lock() {
            responses.insert(response.request_id.clone(), response);
        }
    });
}

pub fn poll_renderer_probe_requests<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    std::fs::create_dir_all(renderer_probe_bridge_requests_dir())
        .map_err(|error| format!("failed to create probe request dir: {error}"))?;
    std::fs::create_dir_all(renderer_probe_bridge_responses_dir())
        .map_err(|error| format!("failed to create probe response dir: {error}"))?;

    let mut entries = std::fs::read_dir(renderer_probe_bridge_requests_dir())
        .map_err(|error| format!("failed to read probe request dir: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let request_path = entry.path();
        let request_content = std::fs::read_to_string(&request_path)
            .map_err(|error| format!("failed to read probe request: {error}"))?;
        let request = match serde_json::from_str::<RendererProbeBridgeRequest>(&request_content) {
            Ok(request) => request,
            Err(_) => {
                let _ = std::fs::remove_file(&request_path);
                continue;
            }
        };

        let result = if app.get_webview_window("main").is_none() {
            RendererProbeResult::Failure {
                reason: RendererProbeFailureReason::NoMainWindow,
            }
        } else {
            request_renderer_probe(app, &request)?
        };

        let response = RendererProbeBridgeResponse {
            request_id: request.request_id.clone(),
            requested_at: request.requested_at.clone(),
            responded_at: crate::inspection::screenshot::timestamp_now(),
            result,
        };

        let response_path = renderer_probe_bridge_response_path(&request.request_id);
        let response_body = serde_json::to_string_pretty(&response)
            .map_err(|error| format!("failed to serialize probe response: {error}"))?;
        std::fs::write(&response_path, format!("{response_body}\n"))
            .map_err(|error| format!("failed to write probe response: {error}"))?;
        let _ = std::fs::remove_file(&request_path);
    }

    Ok(())
}

fn request_renderer_probe<R: Runtime>(
    app: &AppHandle<R>,
    request: &RendererProbeBridgeRequest,
) -> Result<RendererProbeResult, String> {
    clear_previous_response(app, &request.request_id);

    let payload = RendererProbeEventRequest {
        request_id: request.request_id.clone(),
        requested_at: request.requested_at.clone(),
        probe: request.probe.clone(),
    };

    app.emit_to("main", RENDERER_PROBE_REQUEST_EVENT, payload)
        .map_err(|error| format!("failed to emit probe request event: {error}"))?;

    let started = std::time::SystemTime::now();
    let timeout = Duration::from_secs(10);

    loop {
        if let Some(result) = take_response(app, &request.request_id, &request.requested_at) {
            return Ok(result);
        }

        if started.elapsed().unwrap_or_default() > timeout {
            clear_previous_response(app, &request.request_id);
            return Ok(RendererProbeResult::Failure {
                reason: RendererProbeFailureReason::ProbeTimedOut,
            });
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn take_response<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    requested_at: &str,
) -> Option<RendererProbeResult> {
    let state = app.state::<RendererProbeResponses>();
    let mut responses = state.0.lock().ok()?;
    let response = responses.remove(request_id)?;

    if response.requested_at == requested_at {
        Some(response.result)
    } else {
        None
    }
}

fn clear_previous_response<R: Runtime>(app: &AppHandle<R>, request_id: &str) {
    if let Ok(mut responses) = app.state::<RendererProbeResponses>().0.lock() {
        responses.remove(request_id);
    }
}
