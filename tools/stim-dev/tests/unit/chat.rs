use serde_json::json;
use stim_shared::inspection::{RendererActionFailureReason, RendererActionResult};

use crate::chat::{action_result_json, parse_run_args, percent_encode_path_segment};

#[test]
fn parses_positional_turn() {
    let scenario = parse_run_args(&["hello".into(), "world".into()]).unwrap();

    assert!(!scenario.new_conversation);
    assert_eq!(scenario.turns, vec!["hello world"]);
}

#[test]
fn parses_new_multiturn() {
    let scenario = parse_run_args(&[
        "--new".into(),
        "--turn".into(),
        "first".into(),
        "--turn".into(),
        "second".into(),
    ])
    .unwrap();

    assert!(scenario.new_conversation);
    assert_eq!(scenario.turns, vec!["first", "second"]);
}

#[test]
fn rejects_ambiguous_turns() {
    assert!(parse_run_args(&["--turn".into()]).is_err());
    assert!(parse_run_args(&["--turn".into(), "first".into(), "extra".into()]).is_err());
    assert!(parse_run_args(&["--unknown".into()]).is_err());
}

#[test]
fn shapes_action_failure() {
    let output = action_result_json(RendererActionResult::Failure {
        reason: RendererActionFailureReason::ActionTimedOut,
        detail: Some("waited".into()),
    });

    assert_eq!(
        output,
        json!({
            "state": "failed",
            "reason": "action-timed-out",
            "detail": "waited",
        })
    );
}

#[test]
fn encodes_path_segments() {
    assert_eq!(percent_encode_path_segment("conv/a b"), "conv%2Fa%20b");
}
