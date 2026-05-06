use std::time::Duration;

use stim_shared::{
    inspection::{
        ControllerRuntimeBridgeRequest, ControllerRuntimeBridgeResponse, InspectBridgeRequest,
        InspectBridgeResponse, InspectResult, RendererActionBridgeRequest,
        RendererActionBridgeResponse, RendererActionRequest, RendererActionResult,
        RendererProbeBridgeRequest, RendererProbeBridgeResponse, RendererProbeRequest,
        RendererProbeResult, ScreenshotBridgeRequest, ScreenshotBridgeResponse, ScreenshotResult,
    },
    paths::{
        controller_runtime_bridge_request_path, controller_runtime_bridge_response_path,
        inspect_bridge_request_path, inspect_bridge_response_path,
        renderer_action_bridge_request_path, renderer_action_bridge_response_path,
        renderer_probe_bridge_request_path, renderer_probe_bridge_response_path,
        screenshot_bridge_request_path, screenshot_bridge_response_path,
    },
};

use crate::shared::clock::{create_request_id, timestamp_now};

mod exchange;

use exchange::{request_bridge_response, BridgeExchange};

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

pub(crate) fn request_renderer_action(
    action: RendererActionRequest,
) -> Result<RendererActionResult, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let timeout = renderer_action_timeout(&action);
    let request = RendererActionBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        action,
    };

    let response = request_bridge_response::<_, RendererActionBridgeResponse>(BridgeExchange {
        label: "renderer action",
        request_path: renderer_action_bridge_request_path(&request_id),
        response_path: renderer_action_bridge_response_path(&request_id),
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

fn renderer_action_timeout(action: &RendererActionRequest) -> Duration {
    match action {
        RendererActionRequest::MessagingNewConversation => Duration::from_secs(10),
        RendererActionRequest::MessagingSend { .. } => Duration::from_secs(140),
    }
}

#[cfg(test)]
mod tests {
    use super::{renderer_action_timeout, renderer_probe_timeout};
    use std::time::Duration;
    use stim_shared::inspection::{RendererActionRequest, RendererProbeRequest};

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

    #[test]
    fn renderer_actions_have_bounded_smoke_timeout() {
        assert_eq!(
            renderer_action_timeout(&RendererActionRequest::MessagingSend {
                text: "hello".into(),
                target_endpoint_id: None,
            }),
            Duration::from_secs(140)
        );
    }
}
