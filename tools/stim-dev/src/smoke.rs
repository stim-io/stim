use std::time::Duration;

use stim_shared::inspection::{
    RendererActionFailureReason, RendererActionRequest, RendererActionResult,
    RendererActionSnapshot, RendererMessagingSendSnapshot, RendererMessagingStateSnapshot,
};

use crate::{
    bridge::{request_controller_runtime_with_timeout, request_renderer_action},
    clock::timestamp_now,
    runtime_control::current_namespace,
};

const CONTINUATION_FOLLOWUP_TEXT: &str =
    "What exact text did I send in my previous user message? Quote it verbatim.";

pub(crate) fn smoke(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [target, leaf] if target == "renderer" && leaf == "continuation" => {
            smoke_renderer_continuation(None)
        }
        [target, leaf, text @ ..] if target == "renderer" && leaf == "continuation" => {
            smoke_renderer_continuation(Some(text.join(" ")))
        }
        [target, leaf] if target == "renderer" && leaf == "messaging" => {
            smoke_renderer_messaging(None)
        }
        [target, leaf, text @ ..] if target == "renderer" && leaf == "messaging" => {
            smoke_renderer_messaging(Some(text.join(" ")))
        }
        [] | [_] => Err("smoke requires '<target> <leaf>'; supported leaves: renderer messaging [text], renderer continuation [text]".into()),
        [target, ..] => Err(format!(
            "unsupported smoke leaf under target '{target}'; supported leaves: renderer messaging [text], renderer continuation [text]"
        )),
    }
}

fn smoke_renderer_continuation(text: Option<String>) -> Result<(), String> {
    let controller_runtime = require_running_controller_for_renderer_smoke()?;
    let marker_text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev renderer continuation {}", timestamp_now()));
    let new_conversation =
        request_renderer_action(RendererActionRequest::MessagingNewConversation)?;
    let _new_snapshot = require_action_success(new_conversation.clone(), "new-conversation")?;
    let first_turn = request_renderer_action(RendererActionRequest::MessagingSend {
        text: marker_text.clone(),
        target_endpoint_id: Some("endpoint-b".into()),
    })?;
    let first_turn_snapshot = require_messaging_send_snapshot(first_turn.clone(), "first-turn")?;
    assert_renderer_message_state(
        &first_turn_snapshot.after,
        &marker_text,
        None,
        1,
        1,
        "first-turn",
    )?;

    let second_turn = request_renderer_action(RendererActionRequest::MessagingSend {
        text: CONTINUATION_FOLLOWUP_TEXT.into(),
        target_endpoint_id: Some("endpoint-b".into()),
    })?;
    let second_turn_snapshot = require_messaging_send_snapshot(second_turn.clone(), "second-turn")?;
    assert_renderer_message_state(
        &second_turn_snapshot.after,
        CONTINUATION_FOLLOWUP_TEXT,
        Some(&marker_text),
        2,
        2,
        "second-turn",
    )?;

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": current_namespace(),
        "command": "stim-dev smoke renderer continuation",
        "controller": controller_runtime.snapshot,
        "state": "passed",
        "marker_text": marker_text,
        "followup_text": CONTINUATION_FOLLOWUP_TEXT,
        "new_conversation": action_result_json(new_conversation),
        "first_turn": action_result_json(first_turn),
        "second_turn": action_result_json(second_turn),
    }))
    .map_err(|error| format!("failed to serialize renderer continuation smoke result: {error}"))?;

    println!("{output}");
    Ok(())
}

fn smoke_renderer_messaging(text: Option<String>) -> Result<(), String> {
    let controller_runtime = require_running_controller_for_renderer_smoke()?;
    let text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev renderer smoke {}", timestamp_now()));
    let result = request_renderer_action(RendererActionRequest::MessagingSend {
        text: text.clone(),
        target_endpoint_id: Some("endpoint-b".into()),
    })?;
    let passed = action_result_passed(&result);

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": current_namespace(),
        "command": "stim-dev smoke renderer messaging",
        "controller": controller_runtime.snapshot,
        "submitted_text": text,
        "result": action_result_json(result),
    }))
    .map_err(|error| format!("failed to serialize renderer messaging smoke result: {error}"))?;

    println!("{output}");
    if !passed {
        return Err("renderer messaging smoke failed; see JSON output".into());
    }

    Ok(())
}

fn require_running_controller_for_renderer_smoke(
) -> Result<stim_shared::inspection::ControllerRuntimeBridgeResponse, String> {
    request_controller_runtime_with_timeout(Duration::from_secs(5)).map_err(|error| {
        format!(
            "renderer smoke requires a running app loop; run 'stim-dev detect' and 'stim-dev restart' first: {error}"
        )
    })
}

fn require_action_success(
    result: RendererActionResult,
    label: &str,
) -> Result<RendererActionSnapshot, String> {
    match result {
        RendererActionResult::Success { snapshot } => Ok(snapshot),
        RendererActionResult::Failure { reason, detail } => Err(format!(
            "renderer {label} action failed: {}{}",
            action_failure_reason_name(reason),
            detail
                .map(|value| format!(" ({value})"))
                .unwrap_or_default()
        )),
    }
}

fn require_messaging_send_snapshot(
    result: RendererActionResult,
    label: &str,
) -> Result<RendererMessagingSendSnapshot, String> {
    match require_action_success(result, label)? {
        RendererActionSnapshot::MessagingSend(snapshot) => Ok(snapshot),
        RendererActionSnapshot::MessagingNewConversation(_) => Err(format!(
            "renderer {label} action returned new-conversation snapshot, expected messaging-send"
        )),
    }
}

fn assert_renderer_message_state(
    snapshot: &RendererMessagingStateSnapshot,
    expected_last_user_text: &str,
    expected_assistant_text_fragment: Option<&str>,
    min_user_entries: usize,
    min_assistant_entries: usize,
    label: &str,
) -> Result<(), String> {
    if let Some(error) = snapshot.error_message.as_deref() {
        return Err(format!("renderer {label} reported visible error: {error}"));
    }
    if snapshot.active_conversation_id.is_none() {
        return Err(format!(
            "renderer {label} did not expose an active conversation"
        ));
    }
    if snapshot.user_entry_count < min_user_entries
        || snapshot.assistant_entry_count < min_assistant_entries
    {
        return Err(format!(
            "renderer {label} had insufficient visible messages: users={}, assistants={}, expected at least users={}, assistants={}",
            snapshot.user_entry_count,
            snapshot.assistant_entry_count,
            min_user_entries,
            min_assistant_entries,
        ));
    }
    if !snapshot
        .last_user_text
        .as_deref()
        .is_some_and(|text| text.contains(expected_last_user_text))
    {
        return Err(format!(
            "renderer {label} last user text did not include expected text '{expected_last_user_text}'"
        ));
    }
    if let Some(expected_assistant_text_fragment) = expected_assistant_text_fragment {
        if !snapshot
            .last_assistant_text
            .as_deref()
            .is_some_and(|text| text.contains(expected_assistant_text_fragment))
        {
            return Err(format!(
                "renderer {label} last assistant text did not include expected text '{expected_assistant_text_fragment}'"
            ));
        }
    } else if snapshot
        .last_assistant_text
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err(format!(
            "renderer {label} did not expose a visible assistant reply"
        ));
    }

    Ok(())
}

fn action_result_passed(result: &RendererActionResult) -> bool {
    matches!(result, RendererActionResult::Success { .. })
}

fn action_result_json(result: RendererActionResult) -> serde_json::Value {
    match result {
        RendererActionResult::Success { snapshot } => {
            serde_json::json!({ "state": "passed", "snapshot": snapshot })
        }
        RendererActionResult::Failure { reason, detail } => serde_json::json!({
            "state": "failed",
            "reason": action_failure_reason_name(reason),
            "detail": detail,
        }),
    }
}

fn action_failure_reason_name(reason: RendererActionFailureReason) -> &'static str {
    match reason {
        RendererActionFailureReason::NoMainWindow => "no-main-window",
        RendererActionFailureReason::ActionFailed => "action-failed",
        RendererActionFailureReason::ActionTimedOut => "action-timed-out",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use stim_shared::inspection::{
        RendererActionFailureReason, RendererActionResult, RendererMessagingStateSnapshot,
    };

    use super::{action_result_json, action_result_passed, assert_renderer_message_state, smoke};

    #[test]
    fn smoke_rejects_unknown_or_incomplete_leaves() {
        assert!(smoke(Vec::new()).unwrap_err().contains("smoke requires"));
        assert!(smoke(vec!["renderer".into()])
            .unwrap_err()
            .contains("smoke requires"));
        assert!(smoke(vec!["tauri".into(), "messaging".into()])
            .unwrap_err()
            .contains("unsupported smoke leaf"));
    }

    #[test]
    fn renderer_continuation_state_requires_visible_marker_reply() {
        let snapshot = RendererMessagingStateSnapshot {
            document_ready_state: "complete".into(),
            active_session_id: Some("live-controller".into()),
            active_conversation_id: Some("conv-1".into()),
            chat_entry_count: 4,
            user_entry_count: 2,
            assistant_entry_count: 2,
            last_user_text: Some(super::CONTINUATION_FOLLOWUP_TEXT.into()),
            last_assistant_text: Some("marker kiwi lantern".into()),
            response_text: Some("marker kiwi lantern".into()),
            response_source: Some("stim_reply_handle".into()),
            final_sent_text: Some(super::CONTINUATION_FOLLOWUP_TEXT.into()),
            assistant_response_content_kind: Some("text".into()),
            assistant_fragment_present: false,
            error_message: None,
            primary_action_label: Some("Send message".into()),
        };

        assert!(assert_renderer_message_state(
            &snapshot,
            super::CONTINUATION_FOLLOWUP_TEXT,
            Some("marker kiwi lantern"),
            2,
            2,
            "second-turn",
        )
        .is_ok());
        assert!(assert_renderer_message_state(
            &snapshot,
            super::CONTINUATION_FOLLOWUP_TEXT,
            Some("missing"),
            2,
            2,
            "second-turn",
        )
        .is_err());
    }

    #[test]
    fn failed_renderer_action_reports_kebab_case_reason() {
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
        assert!(!action_result_passed(&failure));
    }
}
