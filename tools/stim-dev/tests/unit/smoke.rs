use serde_json::json;
use stim_shared::inspection::{
    RendererActionFailureReason, RendererActionResult, RendererMessagingStateSnapshot,
};

use crate::smoke::{
    assertions::assert_renderer_message_state,
    renderer::{action_result_json, CONTINUATION_FOLLOWUP_TEXT},
    smoke,
};

#[test]
fn smoke_rejects_bad_leaves() {
    assert!(smoke(Vec::new()).unwrap_err().contains("smoke requires"));
    assert!(smoke(vec!["renderer".into()])
        .unwrap_err()
        .contains("smoke requires"));
    assert!(smoke(vec!["tauri".into(), "messaging".into()])
        .unwrap_err()
        .contains("unsupported smoke leaf"));
}

#[test]
fn continuation_requires_marker() {
    let snapshot = RendererMessagingStateSnapshot {
        document_ready_state: "complete".into(),
        active_session_id: Some("live-controller".into()),
        active_conversation_id: Some("conv-1".into()),
        chat_entry_count: 4,
        user_entry_count: 2,
        assistant_entry_count: 2,
        last_user_text: Some(CONTINUATION_FOLLOWUP_TEXT.into()),
        last_assistant_text: Some("marker kiwi lantern".into()),
        response_text: Some("marker kiwi lantern".into()),
        response_source: Some("stim_reply_handle".into()),
        final_sent_text: Some(CONTINUATION_FOLLOWUP_TEXT.into()),
        tool_activity_count: 0,
        latest_tool_activity_summary: None,
        assistant_response_content_kind: Some("text".into()),
        assistant_fragment_present: false,
        error_message: None,
        primary_action_label: Some("Send message".into()),
    };

    assert!(assert_renderer_message_state(
        &snapshot,
        CONTINUATION_FOLLOWUP_TEXT,
        Some("marker kiwi lantern"),
        2,
        2,
        "second-turn",
    )
    .is_ok());
    assert!(assert_renderer_message_state(
        &snapshot,
        CONTINUATION_FOLLOWUP_TEXT,
        Some("missing"),
        2,
        2,
        "second-turn",
    )
    .is_err());
}

#[test]
fn failed_action_reports_reason() {
    let failure = RendererActionResult::Failure {
        reason: RendererActionFailureReason::ActionTimedOut,
        detail: Some("timed out".into()),
    };
    let output = action_result_json(failure.clone());

    assert_eq!(
        output,
        json!({
            "state": "failed",
            "reason": "action-timed-out",
            "detail": "timed out",
        })
    );
}
