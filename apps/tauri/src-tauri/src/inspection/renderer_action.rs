use std::{collections::HashMap, sync::Mutex, thread, time::Duration};

use tauri::{AppHandle, Emitter, Listener, Manager, Runtime};

use stim_shared::{
    inspection::{
        RendererActionBridgeRequest, RendererActionBridgeResponse, RendererActionEventRequest,
        RendererActionEventResponse, RendererActionFailureReason, RendererActionRequest,
        RendererActionResult, RENDERER_ACTION_REQUEST_EVENT, RENDERER_ACTION_RESPONSE_EVENT,
    },
    paths::{
        renderer_action_bridge_requests_dir, renderer_action_bridge_response_path,
        renderer_action_bridge_responses_dir,
    },
};

pub struct RendererActionResponses(pub Mutex<HashMap<String, RendererActionEventResponse>>);

impl Default for RendererActionResponses {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

pub fn manage_renderer_action_state<R: Runtime>(app: &mut tauri::App<R>) {
    app.manage(RendererActionResponses::default());
}

pub fn register_renderer_action_listener<R: Runtime>(app: &AppHandle<R>) {
    let app_handle = app.clone();
    app.listen_any(RENDERER_ACTION_RESPONSE_EVENT, move |event| {
        let Ok(response) = serde_json::from_str::<RendererActionEventResponse>(event.payload())
        else {
            return;
        };

        if let Ok(mut responses) = app_handle.state::<RendererActionResponses>().0.lock() {
            responses.insert(response.request_id.clone(), response);
        }
    });
}

pub fn poll_renderer_action_requests<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    std::fs::create_dir_all(renderer_action_bridge_requests_dir())
        .map_err(|error| format!("failed to create action request dir: {error}"))?;
    std::fs::create_dir_all(renderer_action_bridge_responses_dir())
        .map_err(|error| format!("failed to create action response dir: {error}"))?;

    let mut entries = std::fs::read_dir(renderer_action_bridge_requests_dir())
        .map_err(|error| format!("failed to read action request dir: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let request_path = entry.path();
        let request_content = std::fs::read_to_string(&request_path)
            .map_err(|error| format!("failed to read action request: {error}"))?;
        let request = match serde_json::from_str::<RendererActionBridgeRequest>(&request_content) {
            Ok(request) => request,
            Err(_) => {
                let _ = std::fs::remove_file(&request_path);
                continue;
            }
        };

        let result = if app.get_webview_window("main").is_none() {
            RendererActionResult::Failure {
                reason: RendererActionFailureReason::NoMainWindow,
                detail: None,
            }
        } else {
            request_renderer_action(app, &request)?
        };

        let response = RendererActionBridgeResponse {
            request_id: request.request_id.clone(),
            requested_at: request.requested_at.clone(),
            responded_at: crate::inspection::screenshot::timestamp_now(),
            result,
        };

        let response_path = renderer_action_bridge_response_path(&request.request_id);
        let response_body = serde_json::to_string_pretty(&response)
            .map_err(|error| format!("failed to serialize action response: {error}"))?;
        std::fs::write(&response_path, format!("{response_body}\n"))
            .map_err(|error| format!("failed to write action response: {error}"))?;
        let _ = std::fs::remove_file(&request_path);
    }

    Ok(())
}

fn request_renderer_action<R: Runtime>(
    app: &AppHandle<R>,
    request: &RendererActionBridgeRequest,
) -> Result<RendererActionResult, String> {
    clear_previous_response(app, &request.request_id);

    let payload = RendererActionEventRequest {
        request_id: request.request_id.clone(),
        requested_at: request.requested_at.clone(),
        action: request.action.clone(),
    };

    app.emit_to("main", RENDERER_ACTION_REQUEST_EVENT, payload)
        .map_err(|error| format!("failed to emit action request event: {error}"))?;

    let started = std::time::SystemTime::now();
    let timeout = renderer_action_timeout(&request.action);

    loop {
        if let Some(result) = take_response(app, &request.request_id, &request.requested_at) {
            return Ok(result);
        }

        if started.elapsed().unwrap_or_default() > timeout {
            clear_previous_response(app, &request.request_id);
            return Ok(RendererActionResult::Failure {
                reason: RendererActionFailureReason::ActionTimedOut,
                detail: None,
            });
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn renderer_action_timeout(action: &RendererActionRequest) -> Duration {
    match action {
        RendererActionRequest::MessagingSend { .. } => Duration::from_secs(60),
    }
}

fn take_response<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    requested_at: &str,
) -> Option<RendererActionResult> {
    let state = app.state::<RendererActionResponses>();
    let mut responses = state.0.lock().ok()?;
    let response = responses.remove(request_id)?;

    if response.requested_at == requested_at {
        Some(response.result)
    } else {
        None
    }
}

fn clear_previous_response<R: Runtime>(app: &AppHandle<R>, request_id: &str) {
    if let Ok(mut responses) = app.state::<RendererActionResponses>().0.lock() {
        responses.remove(request_id);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use stim_shared::inspection::RendererActionRequest;

    use super::renderer_action_timeout;

    #[test]
    fn renderer_action_timeout_allows_real_roundtrip_without_becoming_unbounded() {
        assert_eq!(
            renderer_action_timeout(&RendererActionRequest::MessagingSend {
                text: "hello".into(),
                target_endpoint_id: None,
            }),
            Duration::from_secs(60)
        );
    }
}
