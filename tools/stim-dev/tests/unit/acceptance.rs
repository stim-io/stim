use stim_shared::message_operation::{
    ControllerOperationEvent, ControllerOperationMessage, ControllerOperationSnapshot,
    ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

use crate::acceptance::{
    accept,
    controller::{
        assertions::{
            assert_last_assistant_contains, assert_snapshot_message_counts,
            assert_snapshot_user_texts,
        },
        operation_socket::{controller_operation_ws_url, require_completed_snapshot},
    },
};

#[test]
fn accept_rejects_bad_leaves() {
    assert!(accept(Vec::new()).unwrap_err().contains("accept requires"));
    assert!(accept(vec!["controller".into()])
        .unwrap_err()
        .contains("accept requires"));
    assert!(accept(vec!["renderer".into(), "messaging".into()])
        .unwrap_err()
        .contains("unsupported accept leaf"));
}

#[test]
fn requires_distinct_turns() {
    let snapshot = ControllerOperationSnapshot {
        conversation_id: "conv-1".into(),
        message_count: 4,
        user_message_count: 2,
        assistant_message_count: 2,
        tool_activity_count: 0,
        tool_result_count: 0,
        last_user_text: Some("second".into()),
        last_assistant_text: Some("the prior text was first".into()),
        final_sent_text: Some("second".into()),
        response_text_source: Some("stim_reply_handle".into()),
        messages: vec![
            ControllerOperationMessage {
                id: "msg-1".into(),
                role: "user".into(),
                text: "first".into(),
            },
            ControllerOperationMessage {
                id: "msg-2".into(),
                role: "assistant".into(),
                text: "assistant one".into(),
            },
            ControllerOperationMessage {
                id: "msg-3".into(),
                role: "user".into(),
                text: "second".into(),
            },
            ControllerOperationMessage {
                id: "msg-4".into(),
                role: "assistant".into(),
                text: "the prior text was first".into(),
            },
        ],
        tool_activities: vec![],
    };

    assert!(assert_snapshot_user_texts(&snapshot, &["first", "second"], "final").is_ok());
    assert!(assert_snapshot_message_counts(&snapshot, 2, 2, "final").is_ok());
    assert!(assert_last_assistant_contains(&snapshot, "first", "final").is_ok());
    assert!(assert_snapshot_user_texts(&snapshot, &["missing"], "final").is_err());
    assert!(assert_last_assistant_contains(&snapshot, "missing", "final").is_err());

    let mut empty_snapshot = snapshot.clone();
    for message in &mut empty_snapshot.messages {
        if message.role == "assistant" {
            message.text.clear();
        }
    }
    assert!(assert_snapshot_message_counts(&empty_snapshot, 2, 2, "final").is_err());
}

#[test]
fn ws_uses_service_transport() {
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
fn terminal_failure_fails() {
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
        references: vec![],
        message_delta: None,
        snapshot: None,
    }];

    assert!(require_completed_snapshot(&events, "send-text")
        .unwrap_err()
        .contains("boom"));
}
