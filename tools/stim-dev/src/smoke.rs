use std::time::Duration;

use stim_shared::inspection::{
    RendererActionFailureReason, RendererActionRequest, RendererActionResult,
};

use crate::{
    bridge::{request_controller_runtime_with_timeout, request_renderer_action},
    clock::timestamp_now,
    runtime_control::current_namespace,
};

pub(crate) fn smoke(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [target, leaf] if target == "renderer" && leaf == "messaging" => {
            smoke_renderer_messaging(None)
        }
        [target, leaf, text @ ..] if target == "renderer" && leaf == "messaging" => {
            smoke_renderer_messaging(Some(text.join(" ")))
        }
        [] | [_] => Err("smoke requires '<target> <leaf>'; supported leaf: renderer messaging [text]".into()),
        [target, ..] => Err(format!(
            "unsupported smoke leaf under target '{target}'; supported leaf: renderer messaging [text]"
        )),
    }
}

fn smoke_renderer_messaging(text: Option<String>) -> Result<(), String> {
    let controller_runtime = request_controller_runtime_with_timeout(Duration::from_secs(5))
        .map_err(|error| {
            format!(
                "renderer messaging smoke requires a running app loop; run 'stim-dev detect' and 'stim-dev restart' first: {error}"
            )
        })?;
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

    use super::{action_result_json, action_result_passed, smoke};

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
