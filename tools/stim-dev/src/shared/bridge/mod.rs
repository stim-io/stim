use std::time::Duration;

use stim_shared::{
    inspection::{
        AgentsRuntimeBridgeRequest, AgentsRuntimeBridgeResponse, ControllerRuntimeBridgeRequest,
        ControllerRuntimeBridgeResponse, InspectBridgeRequest, InspectBridgeResponse,
        InspectResult, RendererActionBridgeRequest, RendererActionBridgeResponse,
        RendererActionRequest, RendererActionResult, RendererProbeBridgeRequest,
        RendererProbeBridgeResponse, RendererProbeRequest, RendererProbeResult,
        ScreenshotBridgeRequest, ScreenshotBridgeResponse, ScreenshotResult,
    },
    paths::{
        agents_runtime_request_path, agents_runtime_response_path, controller_runtime_request_path,
        controller_runtime_response_path, inspect_bridge_request_path,
        inspect_bridge_response_path, renderer_action_request_path, renderer_action_response_path,
        renderer_probe_request_path, renderer_probe_response_path, screenshot_bridge_request_path,
        screenshot_bridge_response_path,
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
        request_path: renderer_probe_request_path(&request_id),
        response_path: renderer_probe_response_path(&request_id),
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
        request_path: renderer_action_request_path(&request_id),
        response_path: renderer_action_response_path(&request_id),
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

pub(crate) fn request_controller_runtime(
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
        request_path: controller_runtime_request_path(&request_id),
        response_path: controller_runtime_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout,
    })
}

pub(crate) fn request_agents_runtime(
    timeout: Duration,
) -> Result<AgentsRuntimeBridgeResponse, String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = AgentsRuntimeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    request_bridge_response::<_, AgentsRuntimeBridgeResponse>(BridgeExchange {
        label: "agents runtime",
        request_path: agents_runtime_request_path(&request_id),
        response_path: agents_runtime_response_path(&request_id),
        request: &request,
        request_id: &request_id,
        requested_at: &requested_at,
        timeout,
    })
}

pub(crate) fn renderer_probe_timeout(probe: &RendererProbeRequest) -> Duration {
    match probe {
        RendererProbeRequest::LandingBasics => Duration::from_secs(10),
        RendererProbeRequest::MessagingState => Duration::from_secs(10),
    }
}

pub(crate) fn renderer_action_timeout(action: &RendererActionRequest) -> Duration {
    match action {
        RendererActionRequest::MessagingNewConversation => Duration::from_secs(10),
        RendererActionRequest::MessagingSend { .. } => Duration::from_secs(140),
    }
}
