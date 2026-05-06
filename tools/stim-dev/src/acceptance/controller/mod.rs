use crate::{control::current_namespace, shared::clock::timestamp_now};

mod assertions;
mod operation_socket;

use assertions::{
    assert_last_assistant_contains, assert_snapshot_contains_user_texts,
    assert_snapshot_conversation, assert_snapshot_has_tool_activity,
    assert_snapshot_message_counts,
};
use operation_socket::{
    execute_load_transcript, execute_send_text, start_controller_for_acceptance,
    stop_controller_processes, stop_result_json,
};

const TOOL_ACTIVITY_ACCEPTANCE_TEXT: &str =
    "请调用一次 bash 工具执行只读命令 `pwd`，然后用一句话说明工具已完成。";

pub(super) fn accept_messaging(text: Option<String>) -> Result<(), String> {
    let namespace = current_namespace();
    let first_text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("stim-dev controller acceptance {}", timestamp_now()));
    let second_text =
        "What exact text did I send in my previous user message? Quote it verbatim.".to_string();
    let result = run_controller_messaging_acceptance(&namespace, &first_text, &second_text);
    finish_controller_acceptance(
        &namespace,
        result,
        "controller acceptance",
        "controller acceptance report",
    )
}

pub(super) fn accept_tool_activity(text: Option<String>) -> Result<(), String> {
    let namespace = current_namespace();
    let text = text
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| TOOL_ACTIVITY_ACCEPTANCE_TEXT.to_string());
    let result = run_controller_tool_activity_acceptance(&namespace, &text);
    finish_controller_acceptance(
        &namespace,
        result,
        "controller tool activity",
        "controller tool activity report",
    )
}

fn finish_controller_acceptance(
    namespace: &str,
    result: Result<serde_json::Value, String>,
    cleanup_context: &str,
    report_context: &str,
) -> Result<(), String> {
    let final_stop = stop_controller_processes(namespace);

    let report = match (result, final_stop) {
        (Ok(mut report), Ok(stop)) => {
            report["final_controller_stop"] = stop_result_json(&stop);
            report
        }
        (Err(error), Ok(_)) => return Err(error),
        (Ok(_), Err(error)) => {
            return Err(format!("{cleanup_context} cleanup failed: {error}"));
        }
        (Err(error), Err(cleanup_error)) => {
            return Err(format!(
                "{error}; {cleanup_context} cleanup also failed: {cleanup_error}"
            ));
        }
    };

    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize {report_context}: {error}"))?;
    println!("{output}");
    Ok(())
}

fn run_controller_tool_activity_acceptance(
    namespace: &str,
    text: &str,
) -> Result<serde_json::Value, String> {
    let initial_stop = stop_controller_processes(namespace)?;
    let controller = start_controller_for_acceptance(namespace)?;
    let (send_events, send_snapshot) = execute_send_text(&controller.endpoint, text, None, "send")?;
    assert_snapshot_contains_user_texts(&send_snapshot, &[text], "send")?;
    assert_snapshot_message_counts(&send_snapshot, 1, 1, "send")?;
    assert_snapshot_has_tool_activity(&send_snapshot, "send")?;

    Ok(serde_json::json!({
        "namespace": namespace,
        "command": "stim-dev accept controller tool-activity",
        "state": "passed",
        "submitted_text": text,
        "conversation_id": send_snapshot.conversation_id,
        "initial_controller_stop": stop_result_json(&initial_stop),
        "controller": controller,
        "send": {
            "events": send_events,
            "snapshot": send_snapshot,
        },
    }))
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
