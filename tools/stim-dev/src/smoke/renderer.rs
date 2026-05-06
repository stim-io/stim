use std::time::Duration;

use stim_shared::inspection::{
    RendererActionFailureReason, RendererActionRequest, RendererActionResult,
    RendererActionSnapshot, RendererMessagingSendSnapshot,
};

use crate::{
    control::current_namespace,
    shared::{
        bridge::{request_controller_runtime_with_timeout, request_renderer_action},
        clock::timestamp_now,
    },
};

use super::assertions::assert_renderer_message_state;

pub(super) const CONTINUATION_FOLLOWUP_TEXT: &str =
    "What exact text did I send in my previous user message? Quote it verbatim.";

pub(super) fn smoke_continuation(text: Option<String>) -> Result<(), String> {
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

pub(super) fn smoke_messaging(text: Option<String>) -> Result<(), String> {
    let controller_runtime = require_running_controller_for_renderer_smoke()?;
    let text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev renderer smoke {}", timestamp_now()));
    let result = request_renderer_action(RendererActionRequest::MessagingSend {
        text: text.clone(),
        target_endpoint_id: Some("endpoint-b".into()),
    })?;
    let state_check =
        require_messaging_send_snapshot(result.clone(), "messaging").and_then(|snapshot| {
            assert_renderer_message_state(&snapshot.after, &text, None, 1, 1, "messaging")
        });

    let output = serde_json::to_string_pretty(&serde_json::json!({
        "namespace": current_namespace(),
        "command": "stim-dev smoke renderer messaging",
        "controller": controller_runtime.snapshot,
        "submitted_text": text,
        "result": action_result_json(result),
    }))
    .map_err(|error| format!("failed to serialize renderer messaging smoke result: {error}"))?;

    println!("{output}");
    if let Err(error) = state_check {
        return Err(format!(
            "renderer messaging smoke failed: {error}; see JSON output"
        ));
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
        RendererActionResult::Success { snapshot } => Ok(*snapshot),
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

#[cfg(test)]
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
    use stim_shared::inspection::{RendererActionFailureReason, RendererActionResult};

    use super::{action_result_json, action_result_passed};

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
