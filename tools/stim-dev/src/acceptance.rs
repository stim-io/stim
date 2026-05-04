use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationSnapshot, ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};
use stim_sidecar::process::StampedProcessCriteria;
use tungstenite::Message;

use crate::{
    clock::{create_request_id, timestamp_now},
    runtime_control::{current_namespace, stop_matching_processes},
    sidecars::spawn_controller_ready_detached,
};

pub(crate) fn accept(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [target, leaf] if target == "controller" && leaf == "messaging" => {
            accept_controller_messaging(None)
        }
        [target, leaf, text @ ..] if target == "controller" && leaf == "messaging" => {
            accept_controller_messaging(Some(text.join(" ")))
        }
        [] | [_] => Err(
            "accept requires '<target> <leaf>'; supported leaf: controller messaging [text]"
                .into(),
        ),
        [target, ..] => Err(format!(
            "unsupported accept leaf under target '{target}'; supported leaf: controller messaging [text]"
        )),
    }
}

fn accept_controller_messaging(text: Option<String>) -> Result<(), String> {
    let namespace = current_namespace();
    let text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev controller acceptance {}", timestamp_now()));
    let result = run_controller_messaging_acceptance(&namespace, &text);
    let final_stop = stop_controller_processes(&namespace);

    let report = match (result, final_stop) {
        (Ok(mut report), Ok(stop)) => {
            report["final_controller_stop"] = stop_result_json(&stop);
            report
        }
        (Err(error), Ok(_)) => return Err(error),
        (Ok(_), Err(error)) => {
            return Err(format!("controller acceptance cleanup failed: {error}"))
        }
        (Err(error), Err(cleanup_error)) => {
            return Err(format!(
                "{error}; controller acceptance cleanup also failed: {cleanup_error}"
            ));
        }
    };

    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize controller acceptance report: {error}"))?;
    println!("{output}");
    Ok(())
}

fn run_controller_messaging_acceptance(
    namespace: &str,
    text: &str,
) -> Result<serde_json::Value, String> {
    let initial_stop = stop_controller_processes(namespace)?;
    let first_controller = start_controller_for_acceptance(namespace)?;
    let send_command = command_envelope(ControllerOperationCommand::SendText {
        text: text.into(),
        target_endpoint_id: "endpoint-b".into(),
        conversation_id: None,
    });
    let send_events = execute_operation_command(&first_controller.endpoint, &send_command)?;
    let send_snapshot = require_completed_snapshot(&send_events, "send-text")?;
    assert_snapshot_contains_user_text(&send_snapshot, text, "send-text")?;
    assert_snapshot_has_assistant(&send_snapshot, "send-text")?;
    let conversation_id = send_snapshot.conversation_id.clone();

    let restart_stop = stop_controller_processes(namespace)?;
    let second_controller = start_controller_for_acceptance(namespace)?;
    let reload_command = command_envelope(ControllerOperationCommand::LoadTranscript {
        conversation_id: conversation_id.clone(),
    });
    let reload_events = execute_operation_command(&second_controller.endpoint, &reload_command)?;
    let reload_snapshot = require_completed_snapshot(&reload_events, "load-transcript")?;
    assert_snapshot_contains_user_text(&reload_snapshot, text, "load-transcript")?;
    assert_snapshot_has_assistant(&reload_snapshot, "load-transcript")?;

    Ok(serde_json::json!({
        "namespace": namespace,
        "command": "stim-dev accept controller messaging",
        "state": "passed",
        "submitted_text": text,
        "conversation_id": conversation_id,
        "initial_controller_stop": stop_result_json(&initial_stop),
        "first_controller": first_controller,
        "send": {
            "events": send_events,
            "snapshot": send_snapshot,
        },
        "restart_controller_stop": stop_result_json(&restart_stop),
        "second_controller": second_controller,
        "reload": {
            "events": reload_events,
            "snapshot": reload_snapshot,
        },
    }))
}

#[derive(Debug, Clone, serde::Serialize)]
struct AcceptanceController {
    endpoint: String,
    instance_id: String,
}

fn start_controller_for_acceptance(namespace: &str) -> Result<AcceptanceController, String> {
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

fn stop_controller_processes(
    namespace: &str,
) -> Result<stim_platform::process::StopProcessResult, String> {
    let criteria = StampedProcessCriteria {
        app: Some("controller".into()),
        namespace: Some(namespace.into()),
        ..StampedProcessCriteria::default()
    };

    stop_matching_processes(&criteria)
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

fn assert_snapshot_contains_user_text(
    snapshot: &ControllerOperationSnapshot,
    text: &str,
    label: &str,
) -> Result<(), String> {
    if snapshot
        .messages
        .iter()
        .any(|message| message.role == "user" && message.text.contains(text))
    {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot did not contain submitted user text in conversation {}",
        snapshot.conversation_id
    ))
}

fn assert_snapshot_has_assistant(
    snapshot: &ControllerOperationSnapshot,
    label: &str,
) -> Result<(), String> {
    if snapshot.assistant_message_count > 0 && snapshot.last_assistant_text.is_some() {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot did not contain an assistant message in conversation {}",
        snapshot.conversation_id
    ))
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

fn stop_result_json(result: &stim_platform::process::StopProcessResult) -> serde_json::Value {
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

    use super::{accept, controller_operation_ws_url, require_completed_snapshot};

    #[test]
    fn accept_rejects_unknown_or_incomplete_leaves() {
        assert!(accept(Vec::new()).unwrap_err().contains("accept requires"));
        assert!(accept(vec!["controller".into()])
            .unwrap_err()
            .contains("accept requires"));
        assert!(accept(vec!["renderer".into(), "messaging".into()])
            .unwrap_err()
            .contains("unsupported accept leaf"));
    }

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
