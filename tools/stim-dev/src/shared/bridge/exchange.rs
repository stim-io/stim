use std::{fs, path::PathBuf, thread, time::Duration};

use serde::{de::DeserializeOwned, Serialize};
use stim_shared::inspection::{
    AgentsRuntimeBridgeResponse, ControllerRuntimeBridgeResponse, InspectBridgeResponse,
    RendererActionBridgeResponse, RendererProbeBridgeResponse, ScreenshotBridgeResponse,
};

pub(super) trait BridgeResponseEnvelope {
    fn request_id(&self) -> &str;
    fn requested_at(&self) -> &str;
}

impl BridgeResponseEnvelope for ScreenshotBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

impl BridgeResponseEnvelope for RendererProbeBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

impl BridgeResponseEnvelope for RendererActionBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

impl BridgeResponseEnvelope for InspectBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

impl BridgeResponseEnvelope for ControllerRuntimeBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

impl BridgeResponseEnvelope for AgentsRuntimeBridgeResponse {
    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn requested_at(&self) -> &str {
        &self.requested_at
    }
}

pub(super) struct BridgeExchange<'a, Request> {
    pub(super) label: &'a str,
    pub(super) request_path: PathBuf,
    pub(super) response_path: PathBuf,
    pub(super) request: &'a Request,
    pub(super) request_id: &'a str,
    pub(super) requested_at: &'a str,
    pub(super) timeout: Duration,
}

pub(super) fn request_bridge_response<Request, Response>(
    exchange: BridgeExchange<'_, Request>,
) -> Result<Response, String>
where
    Request: Serialize,
    Response: DeserializeOwned + BridgeResponseEnvelope,
{
    create_parent_dir(&exchange.request_path, exchange.label, "request")?;
    create_parent_dir(&exchange.response_path, exchange.label, "response")?;

    let request_body = serde_json::to_string_pretty(exchange.request)
        .map_err(|error| format!("failed to serialize {} request: {error}", exchange.label))?;
    let _ = fs::remove_file(&exchange.response_path);
    fs::write(&exchange.request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write {} request: {error}", exchange.label))?;

    let started = std::time::SystemTime::now();
    loop {
        if started.elapsed().unwrap_or_default() > exchange.timeout {
            cleanup_exchange(&exchange.request_path, &exchange.response_path);
            return Err(format!(
                "timed out waiting for {} response at {}",
                exchange.label,
                exchange.response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&exchange.response_path) {
            let response = serde_json::from_str::<Response>(&content)
                .map_err(|error| format!("failed to parse {} response: {error}", exchange.label))?;

            if response.request_id() == exchange.request_id
                && response.requested_at() == exchange.requested_at
            {
                cleanup_exchange(&exchange.request_path, &exchange.response_path);
                return Ok(response);
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn create_parent_dir(path: &std::path::Path, label: &str, side: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {label} {side} dir: {error}"))?;
    }

    Ok(())
}

fn cleanup_exchange(request_path: &std::path::Path, response_path: &std::path::Path) {
    let _ = fs::remove_file(request_path);
    let _ = fs::remove_file(response_path);
}
