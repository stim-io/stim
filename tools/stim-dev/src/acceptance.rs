use std::time::Duration;

use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationSnapshot, ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};
use stim_sidecar::process::StampedProcessCriteria;
use tungstenite::Message;

const OPERATION_EVENT_READ_TIMEOUT: Duration = Duration::from_secs(60);

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
    let first_text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev controller acceptance {}", timestamp_now()));
    let second_text =
        "What exact text did I send in my previous user message? Quote it verbatim.".to_string();
    let result = run_controller_messaging_acceptance(&namespace, &first_text, &second_text);
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
    first_text: &str,
    second_text: &str,
) -> Result<serde_json::Value, String> {
    let initial_stop = stop_controller_processes(namespace)?;
    let first_controller = start_controller_for_acceptance(namespace)?;
    let (first_send_events, first_send_snapshot) =
        execute_send_text(&first_controller.endpoint, first_text, None, "first-send")?;
    assert_snapshot_contains_user_texts(&first_send_snapshot, &[first_text], "first-send")?;
    assert_snapshot_message_counts(&first_send_snapshot, 1, 1, "first-send")?;
    let conversation_id = first_send_snapshot.conversation_id.clone();

    let restart_stop = stop_controller_processes(namespace)?;
    let second_controller = start_controller_for_acceptance(namespace)?;
    let (reload_before_second_events, reload_before_second_snapshot) = execute_load_transcript(
        &second_controller.endpoint,
        &conversation_id,
        "reload-before-second-turn",
    )?;
    assert_snapshot_conversation(
        &reload_before_second_snapshot,
        &conversation_id,
        "reload-before-second-turn",
    )?;
    assert_snapshot_contains_user_texts(
        &reload_before_second_snapshot,
        &[first_text],
        "reload-before-second-turn",
    )?;
    assert_snapshot_message_counts(
        &reload_before_second_snapshot,
        1,
        1,
        "reload-before-second-turn",
    )?;

    let (second_send_events, second_send_snapshot) = execute_send_text(
        &second_controller.endpoint,
        second_text,
        Some(&conversation_id),
        "second-send",
    )?;
    assert_snapshot_conversation(&second_send_snapshot, &conversation_id, "second-send")?;
    assert_snapshot_contains_user_texts(
        &second_send_snapshot,
        &[first_text, second_text],
        "second-send",
    )?;
    assert_snapshot_message_counts(&second_send_snapshot, 2, 2, "second-send")?;
    assert_last_assistant_contains(&second_send_snapshot, first_text, "second-send")?;

    let final_restart_stop = stop_controller_processes(namespace)?;
    let third_controller = start_controller_for_acceptance(namespace)?;
    let (final_reload_events, final_reload_snapshot) =
        execute_load_transcript(&third_controller.endpoint, &conversation_id, "final-reload")?;
    assert_snapshot_conversation(&final_reload_snapshot, &conversation_id, "final-reload")?;
    assert_snapshot_contains_user_texts(
        &final_reload_snapshot,
        &[first_text, second_text],
        "final-reload",
    )?;
    assert_snapshot_message_counts(&final_reload_snapshot, 2, 2, "final-reload")?;
    assert_last_assistant_contains(&final_reload_snapshot, first_text, "final-reload")?;

    Ok(serde_json::json!({
        "namespace": namespace,
        "command": "stim-dev accept controller messaging",
        "state": "passed",
        "turn_count": 2,
        "submitted_text": first_text,
        "followup_text": second_text,
        "conversation_id": conversation_id,
        "initial_controller_stop": stop_result_json(&initial_stop),
        "first_controller": first_controller,
        "first_turn": {
            "send": {
                "events": first_send_events,
                "snapshot": first_send_snapshot,
            },
        },
        "restart_controller_stop": stop_result_json(&restart_stop),
        "second_controller": second_controller,
        "reload_before_second_turn": {
            "events": reload_before_second_events,
            "snapshot": reload_before_second_snapshot,
        },
        "second_turn": {
            "send": {
                "events": second_send_events,
                "snapshot": second_send_snapshot,
            },
        },
        "final_restart_controller_stop": stop_result_json(&final_restart_stop),
        "third_controller": third_controller,
        "final_reload": {
            "events": final_reload_events,
            "snapshot": final_reload_snapshot,
        },
    }))
}

fn execute_send_text(
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

fn execute_load_transcript(
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

fn assert_snapshot_conversation(
    snapshot: &ControllerOperationSnapshot,
    conversation_id: &str,
    label: &str,
) -> Result<(), String> {
    if snapshot.conversation_id == conversation_id {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot conversation mismatch: expected {conversation_id}, got {}",
        snapshot.conversation_id
    ))
}

fn assert_snapshot_contains_user_texts(
    snapshot: &ControllerOperationSnapshot,
    texts: &[&str],
    label: &str,
) -> Result<(), String> {
    for text in texts {
        if !snapshot
            .messages
            .iter()
            .any(|message| message.role == "user" && message.text == *text)
        {
            return Err(format!(
                "{label} snapshot did not contain submitted user text '{text}' in conversation {}",
                snapshot.conversation_id
            ));
        }
    }

    Ok(())
}

fn assert_snapshot_message_counts(
    snapshot: &ControllerOperationSnapshot,
    min_user_messages: usize,
    min_assistant_messages: usize,
    label: &str,
) -> Result<(), String> {
    let user_message_count = snapshot
        .messages
        .iter()
        .filter(|message| message.role == "user" && !message.text.trim().is_empty())
        .count();
    let assistant_message_count = snapshot
        .messages
        .iter()
        .filter(|message| message.role == "assistant" && !message.text.trim().is_empty())
        .count();

    if user_message_count >= min_user_messages && assistant_message_count >= min_assistant_messages
    {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot had insufficient messages in conversation {}: users={}, assistants={}, expected at least users={}, assistants={}",
        snapshot.conversation_id,
        user_message_count,
        assistant_message_count,
        min_user_messages,
        min_assistant_messages,
    ))
}

fn assert_last_assistant_contains(
    snapshot: &ControllerOperationSnapshot,
    expected_text: &str,
    label: &str,
) -> Result<(), String> {
    let Some(last_assistant_text) = snapshot.last_assistant_text.as_deref() else {
        return Err(format!(
            "{label} snapshot did not contain a last assistant message in conversation {}",
            snapshot.conversation_id
        ));
    };

    if last_assistant_text.contains(expected_text) {
        return Ok(());
    }

    Err(format!(
        "{label} last assistant text did not include expected prior user text '{expected_text}' in conversation {}; actual last assistant text: {last_assistant_text:?}",
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

    use super::{
        accept, assert_last_assistant_contains, assert_snapshot_contains_user_texts,
        assert_snapshot_message_counts, controller_operation_ws_url, require_completed_snapshot,
    };

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

    #[test]
    fn snapshot_assertions_require_distinct_two_turn_content() {
        let snapshot = stim_shared::message_operation::ControllerOperationSnapshot {
            conversation_id: "conv-1".into(),
            message_count: 4,
            user_message_count: 2,
            assistant_message_count: 2,
            last_user_text: Some("second".into()),
            last_assistant_text: Some("the prior text was first".into()),
            final_sent_text: Some("second".into()),
            response_text_source: Some("stim_reply_handle".into()),
            messages: vec![
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-1".into(),
                    role: "user".into(),
                    text: "first".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-2".into(),
                    role: "assistant".into(),
                    text: "assistant one".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-3".into(),
                    role: "user".into(),
                    text: "second".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-4".into(),
                    role: "assistant".into(),
                    text: "the prior text was first".into(),
                },
            ],
        };

        assert!(assert_snapshot_contains_user_texts(
            &snapshot,
            &["first", "second"],
            "final-reload",
        )
        .is_ok());
        assert!(assert_snapshot_message_counts(&snapshot, 2, 2, "final-reload").is_ok());
        assert!(assert_last_assistant_contains(&snapshot, "first", "final-reload").is_ok());
        assert!(
            assert_snapshot_contains_user_texts(&snapshot, &["missing"], "final-reload",).is_err()
        );
        assert!(assert_last_assistant_contains(&snapshot, "missing", "final-reload").is_err());

        let mut empty_assistant_snapshot = snapshot.clone();
        for message in &mut empty_assistant_snapshot.messages {
            if message.role == "assistant" {
                message.text.clear();
            }
        }
        assert!(
            assert_snapshot_message_counts(&empty_assistant_snapshot, 2, 2, "final-reload")
                .is_err()
        );
    }
}
