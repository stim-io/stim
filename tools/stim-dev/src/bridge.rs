use std::{fs, path::PathBuf, thread, time::Duration};

use serde::{de::DeserializeOwned, Serialize};
use stim_shared::{
    inspection::{
        ControllerRuntimeBridgeRequest, ControllerRuntimeBridgeResponse, InspectBridgeRequest,
        InspectBridgeResponse, InspectResult, RendererProbeBridgeRequest,
        RendererProbeBridgeResponse, RendererProbeRequest, RendererProbeResult,
        ScreenshotBridgeRequest, ScreenshotBridgeResponse, ScreenshotResult,
    },
    paths::{
        controller_runtime_bridge_request_path, controller_runtime_bridge_response_path,
        inspect_bridge_request_path, inspect_bridge_response_path,
        renderer_probe_bridge_request_path, renderer_probe_bridge_response_path,
        screenshot_bridge_request_path, screenshot_bridge_response_path,
    },
};

use crate::clock::{create_request_id, timestamp_now};

trait BridgeResponseEnvelope {
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

pub(crate) fn request_screenshot(label: Option<String>) -> Result<ScreenshotResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = ScreenshotBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        label,
    };

    let response = request_bridge_response::<_, ScreenshotBridgeResponse>(BridgeExchange {
        label: "screenshot",
        request_path: screenshot_bridge_request_path(&request_id),
        response_path: screenshot_bridge_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout: Duration::from_secs(15),
    })?;

    Ok(response.result)
}

pub(crate) fn request_probe(probe: RendererProbeRequest) -> Result<RendererProbeResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let timeout = renderer_probe_timeout(&probe);
    let request = RendererProbeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        probe,
    };

    let response = request_bridge_response::<_, RendererProbeBridgeResponse>(BridgeExchange {
        label: "probe",
        request_path: renderer_probe_bridge_request_path(&request_id),
        response_path: renderer_probe_bridge_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout,
    })?;

    Ok(response.result)
}

pub(crate) fn request_inspect() -> Result<InspectResult, String> {
    request_inspect_with_timeout(Duration::from_secs(15))
}

pub(crate) fn request_inspect_with_timeout(timeout: Duration) -> Result<InspectResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = InspectBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    let response = request_bridge_response::<_, InspectBridgeResponse>(BridgeExchange {
        label: "inspect",
        request_path: inspect_bridge_request_path(&request_id),
        response_path: inspect_bridge_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout,
    })?;

    Ok(response.result)
}

pub(crate) fn request_controller_runtime_with_timeout(
    timeout: Duration,
) -> Result<ControllerRuntimeBridgeResponse, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = ControllerRuntimeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    request_bridge_response::<_, ControllerRuntimeBridgeResponse>(BridgeExchange {
        label: "controller runtime",
        request_path: controller_runtime_bridge_request_path(&request_id),
        response_path: controller_runtime_bridge_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout,
    })
}

fn renderer_probe_timeout(probe: &RendererProbeRequest) -> Duration {
    match probe {
        RendererProbeRequest::LandingBasics => Duration::from_secs(10),
        RendererProbeRequest::MessagingState => Duration::from_secs(10),
    }
}

struct BridgeExchange<'a, Request> {
    label: &'a str,
    request_path: PathBuf,
    response_path: PathBuf,
    request: &'a Request,
    request_id: &'a str,
    requested_at: &'a str,
    timeout: Duration,
}

fn request_bridge_response<Request, Response>(
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

#[cfg(test)]
mod tests {
    use super::renderer_probe_timeout;
    use std::time::Duration;
    use stim_shared::inspection::RendererProbeRequest;

    #[test]
    fn renderer_inspect_probes_have_short_timeout_budgets() {
        assert_eq!(
            renderer_probe_timeout(&RendererProbeRequest::LandingBasics),
            Duration::from_secs(10)
        );
        assert_eq!(
            renderer_probe_timeout(&RendererProbeRequest::MessagingState),
            Duration::from_secs(10)
        );
    }
}
