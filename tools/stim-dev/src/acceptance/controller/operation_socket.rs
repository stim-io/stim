use std::time::Duration;

use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationSnapshot, ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};
use stim_sidecar::process::StampedProcessCriteria;
use tungstenite::Message;

use crate::{
    control::stop_matching_processes, shared::clock::create_request_id,
    sidecars::spawn_controller_ready_detached,
};

const OPERATION_EVENT_READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct AcceptanceController {
    pub(super) endpoint: String,
    instance_id: String,
}

pub(super) fn start_controller_for_acceptance(
    namespace: &str,
) -> Result<AcceptanceController, String> {
    let (_child, ready) = spawn_controller_ready_detached(namespace)?;
    let endpoint = ready
        .endpoint
        .clone()
        .ok_or_else(|| "controller ready line did not include endpoint".to_string())?;

    Ok(AcceptanceController {
        endpoint,
        instance_id: ready.instance_id,
    })
}

pub(super) fn stop_controller_processes(
    namespace: &str,
) -> Result<stim_platform::process::StopProcessResult, String> {
    let criteria = StampedProcessCriteria {
        app: Some("controller".into()),
        namespace: Some(namespace.into()),
        ..StampedProcessCriteria::default()
    };

    stop_matching_processes(&criteria)
}

pub(super) fn execute_send_text(
    controller_endpoint: &str,
    text: &str,
    conversation_id: Option<&str>,
    label: &str,
) -> Result<(Vec<ControllerOperationEvent>, ControllerOperationSnapshot), String> {
    let command = command_envelope(ControllerOperationCommand::SendText {
        text: text.into(),
        target_endpoint_id: "endpoint-b".into(),
        conversation_id: conversation_id.map(str::to_string),
    });
    let events = execute_operation_command(controller_endpoint, &command)?;
    let snapshot = require_completed_snapshot(&events, label)?;

    Ok((events, snapshot))
}

pub(super) fn execute_load_transcript(
    controller_endpoint: &str,
    conversation_id: &str,
    label: &str,
) -> Result<(Vec<ControllerOperationEvent>, ControllerOperationSnapshot), String> {
    let command = command_envelope(ControllerOperationCommand::LoadTranscript {
        conversation_id: conversation_id.into(),
    });
    let events = execute_operation_command(controller_endpoint, &command)?;
    let snapshot = require_completed_snapshot(&events, label)?;

    Ok((events, snapshot))
}

fn command_envelope(command: ControllerOperationCommand) -> ControllerOperationCommandEnvelope {
    let id = create_request_id();

    ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: format!("op-{id}"),
        correlation_id: format!("corr-{id}"),
        command,
    }
}

fn execute_operation_command(
    controller_endpoint: &str,
    command: &ControllerOperationCommandEnvelope,
) -> Result<Vec<ControllerOperationEvent>, String> {
    let ws_url = controller_operation_ws_url(controller_endpoint)?;
    let (mut socket, _) = tungstenite::connect(ws_url.as_str())
        .map_err(|error| format!("failed to connect controller operation WebSocket: {error}"))?;
    apply_operation_read_timeout(&mut socket)?;
    let body = serde_json::to_string(command)
        .map_err(|error| format!("failed to serialize controller operation command: {error}"))?;

    socket
        .send(Message::Text(body.into()))
        .map_err(|error| format!("failed to send controller operation command: {error}"))?;

    let mut events = Vec::new();
    loop {
        let message = socket
            .read()
            .map_err(|error| format!("failed to read controller operation event: {error}"))?;
        let Message::Text(text) = message else {
            continue;
        };
        let event = serde_json::from_str::<ControllerOperationEvent>(&text)
            .map_err(|error| format!("failed to decode controller operation event: {error}"))?;
        let terminal = event.is_terminal();
        events.push(event);

        if terminal {
            return Ok(events);
        }
    }
}

fn apply_operation_read_timeout(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) -> Result<(), String> {
    let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() else {
        return Ok(());
    };

    stream
        .set_read_timeout(Some(OPERATION_EVENT_READ_TIMEOUT))
        .map_err(|error| format!("failed to configure controller operation read timeout: {error}"))
}

fn require_completed_snapshot(
    events: &[ControllerOperationEvent],
    label: &str,
) -> Result<ControllerOperationSnapshot, String> {
    let terminal = events
        .last()
        .ok_or_else(|| format!("{label} produced no controller operation events"))?;

    if terminal.stage == ControllerOperationStage::OperationFailed
        || terminal.status == ControllerOperationStatus::Failed
    {
        return Err(format!(
            "{label} failed at controller stage: {}",
            terminal
                .detail
                .clone()
                .unwrap_or_else(|| "no detail".into())
        ));
    }

    if terminal.stage != ControllerOperationStage::OperationCompleted
        || terminal.status != ControllerOperationStatus::Completed
    {
        return Err(format!(
            "{label} ended without operation-completed event: {:?} {:?}",
            terminal.stage, terminal.status
        ));
    }

    terminal
        .snapshot
        .clone()
        .ok_or_else(|| format!("{label} completed without controller snapshot"))
}

fn controller_operation_ws_url(controller_endpoint: &str) -> Result<String, String> {
    let endpoint = controller_endpoint.trim().trim_end_matches('/');
    if let Some(rest) = endpoint.strip_prefix("http://") {
        return Ok(format!("ws://{rest}/api/v1/controller/operations/ws"));
    }
    if let Some(rest) = endpoint.strip_prefix("https://") {
        return Ok(format!("wss://{rest}/api/v1/controller/operations/ws"));
    }

    Err(format!(
        "unsupported controller endpoint scheme for {controller_endpoint}; expected http:// or https://"
    ))
}

pub(super) fn stop_result_json(
    result: &stim_platform::process::StopProcessResult,
) -> serde_json::Value {
    serde_json::json!({
        "already_stopped": result.already_stopped,
        "matched_pids": result.matched_pids,
        "stopped_pids": result.stopped_pids,
        "forced_pids": result.forced_pids,
        "remaining_pids": result.remaining_pids,
    })
}

#[cfg(test)]
mod tests {
    use stim_shared::message_operation::{
        ControllerOperationEvent, ControllerOperationStage, ControllerOperationStatus,
        CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
    };

    use super::{controller_operation_ws_url, require_completed_snapshot};

    #[test]
    fn controller_operation_ws_url_uses_service_transport() {
        assert_eq!(
            controller_operation_ws_url("http://127.0.0.1:18000").unwrap(),
            "ws://127.0.0.1:18000/api/v1/controller/operations/ws"
        );
        assert_eq!(
            controller_operation_ws_url("https://example.test/controller/").unwrap(),
            "wss://example.test/controller/api/v1/controller/operations/ws"
        );
        assert!(controller_operation_ws_url("file:///tmp/socket").is_err());
    }

    #[test]
    fn failed_terminal_event_fails_acceptance() {
        let events = vec![ControllerOperationEvent {
            schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
            event_id: "event-1".into(),
            operation_id: "op-1".into(),
            correlation_id: "corr-1".into(),
            causation_id: None,
            conversation_id: None,
            message_id: None,
            stage: ControllerOperationStage::OperationFailed,
            status: ControllerOperationStatus::Failed,
            occurred_at: "2026-05-04T00:00:00Z".into(),
            detail: Some("boom".into()),
            snapshot: None,
        }];

        assert!(require_completed_snapshot(&events, "send-text")
            .unwrap_err()
            .contains("boom"));
    }
}
