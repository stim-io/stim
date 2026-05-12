mod operation;

use serde_json::{json, Map, Value};

use crate::model::{timestamp_now, ControllerHttpState};

use operation::{
    assert_delivery_target_event, assert_last_assistant_contains, assert_snapshot_conversation,
    assert_snapshot_message_counts, assert_snapshot_tool_activity, assert_snapshot_user_texts,
    execute_load_transcript, execute_send_text, seed_participant_projection,
};

const DEFAULT_TARGET_ENDPOINT_ID: &str = "endpoint-b";
const PARTICIPANT_ROUTING_FALLBACK_ENDPOINT_ID: &str = "participant-resolution-fallback";
const DEFAULT_PARTICIPANT_ID: &str = "santi";
const TOOL_ACTIVITY_ACCEPTANCE_TEXT: &str =
    "请调用一次 bash 工具执行只读命令 `pwd`，然后用一句话说明工具已完成。";
const PARTICIPANT_ROUTING_ACCEPTANCE_TEXT: &str =
    "sidecar participant routing acceptance: reply with a short confirmation.";
const CONTINUATION_FOLLOWUP_TEXT: &str =
    "What exact text did I send in my previous user message? Quote it verbatim.";

pub(crate) async fn run_acceptance_event(
    state: ControllerHttpState,
    verb: &str,
    payload: Value,
) -> Result<Value, String> {
    match verb {
        "accept.messaging" => run_messaging_acceptance(state, payload).await,
        "accept.tool-activity" => run_tool_activity_acceptance(state, payload).await,
        "accept.participant-routing" => run_participant_routing_acceptance(state, payload).await,
        _ => Err(format!("unsupported controller acceptance event {verb}")),
    }
}

async fn run_messaging_acceptance(
    state: ControllerHttpState,
    payload: Value,
) -> Result<Value, String> {
    let request = AcceptancePayload::decode(payload)?;
    let first_text = request
        .text
        .unwrap_or_else(|| format!("sidecar controller acceptance {}", timestamp_now()));
    let followup_text = request
        .followup_text
        .unwrap_or_else(|| CONTINUATION_FOLLOWUP_TEXT.into());
    let target_endpoint_id = request
        .target_endpoint_id
        .unwrap_or_else(|| DEFAULT_TARGET_ENDPOINT_ID.into());

    let first_send = execute_send_text(
        state.clone(),
        first_text.clone(),
        target_endpoint_id.clone(),
        None,
        None,
        "first-send",
    )
    .await?;
    assert_snapshot_user_texts(&first_send.snapshot, &[&first_text], "first-send")?;
    assert_snapshot_message_counts(&first_send.snapshot, 1, 1, "first-send")?;
    let conversation_id = first_send.snapshot.conversation_id.clone();

    let reload_before_second = execute_load_transcript(
        state.clone(),
        conversation_id.clone(),
        "reload-before-second-turn",
    )
    .await?;
    assert_snapshot_conversation(
        &reload_before_second.snapshot,
        &conversation_id,
        "reload-before-second-turn",
    )?;
    assert_snapshot_user_texts(
        &reload_before_second.snapshot,
        &[&first_text],
        "reload-before-second-turn",
    )?;
    assert_snapshot_message_counts(
        &reload_before_second.snapshot,
        1,
        1,
        "reload-before-second-turn",
    )?;

    let second_send = execute_send_text(
        state.clone(),
        followup_text.clone(),
        target_endpoint_id,
        None,
        Some(conversation_id.clone()),
        "second-send",
    )
    .await?;
    assert_snapshot_conversation(&second_send.snapshot, &conversation_id, "second-send")?;
    assert_snapshot_user_texts(
        &second_send.snapshot,
        &[&first_text, &followup_text],
        "second-send",
    )?;
    assert_snapshot_message_counts(&second_send.snapshot, 2, 2, "second-send")?;
    assert_last_assistant_contains(&second_send.snapshot, &first_text, "second-send")?;

    let final_reload =
        execute_load_transcript(state, conversation_id.clone(), "final-reload").await?;
    assert_snapshot_conversation(&final_reload.snapshot, &conversation_id, "final-reload")?;
    assert_snapshot_user_texts(
        &final_reload.snapshot,
        &[&first_text, &followup_text],
        "final-reload",
    )?;
    assert_snapshot_message_counts(&final_reload.snapshot, 2, 2, "final-reload")?;
    assert_last_assistant_contains(&final_reload.snapshot, &first_text, "final-reload")?;

    Ok(json!({
        "command": "sidecar inspect controller accept.messaging",
        "state": "passed",
        "scope": "active-controller-provider",
        "turn_count": 2,
        "submitted_text": first_text,
        "followup_text": followup_text,
        "conversation_id": conversation_id,
        "first_turn": {
            "send": first_send,
        },
        "reload_before_second_turn": reload_before_second,
        "second_turn": {
            "send": second_send,
        },
        "final_reload": final_reload,
    }))
}

async fn run_tool_activity_acceptance(
    state: ControllerHttpState,
    payload: Value,
) -> Result<Value, String> {
    let request = AcceptancePayload::decode(payload)?;
    let text = request
        .text
        .unwrap_or_else(|| TOOL_ACTIVITY_ACCEPTANCE_TEXT.into());
    let target_endpoint_id = request
        .target_endpoint_id
        .unwrap_or_else(|| DEFAULT_TARGET_ENDPOINT_ID.into());

    let send =
        execute_send_text(state, text.clone(), target_endpoint_id, None, None, "send").await?;
    assert_snapshot_user_texts(&send.snapshot, &[&text], "send")?;
    assert_snapshot_message_counts(&send.snapshot, 1, 1, "send")?;
    assert_snapshot_tool_activity(&send.snapshot, "send")?;

    Ok(json!({
        "command": "sidecar inspect controller accept.tool-activity",
        "state": "passed",
        "submitted_text": text,
        "conversation_id": send.snapshot.conversation_id,
        "send": send,
    }))
}

async fn run_participant_routing_acceptance(
    state: ControllerHttpState,
    payload: Value,
) -> Result<Value, String> {
    let request = AcceptancePayload::decode(payload)?;
    let text = request
        .text
        .unwrap_or_else(|| PARTICIPANT_ROUTING_ACCEPTANCE_TEXT.into());
    let participant_id = request
        .participant_id
        .unwrap_or_else(|| DEFAULT_PARTICIPANT_ID.into());
    let delivery_endpoint_id = request
        .delivery_endpoint_id
        .unwrap_or_else(|| DEFAULT_TARGET_ENDPOINT_ID.into());
    let target_endpoint_id = request
        .target_endpoint_id
        .unwrap_or_else(|| PARTICIPANT_ROUTING_FALLBACK_ENDPOINT_ID.into());
    let stim_server_base_url = state.stim_server_base_url.clone();

    seed_participant_projection(
        stim_server_base_url.clone(),
        state.santi_base_url.clone(),
        participant_id.clone(),
        delivery_endpoint_id.clone(),
    )
    .await?;

    let send = execute_send_text(
        state,
        text.clone(),
        target_endpoint_id,
        Some(participant_id.clone()),
        None,
        "participant-send",
    )
    .await?;
    assert_snapshot_user_texts(&send.snapshot, &[&text], "participant-send")?;
    assert_snapshot_message_counts(&send.snapshot, 1, 1, "participant-send")?;
    assert_delivery_target_event(&send.events, &participant_id, &delivery_endpoint_id)?;

    Ok(json!({
        "command": "sidecar inspect controller accept.participant-routing",
        "state": "passed",
        "participant_id": participant_id,
        "delivery_endpoint_id": delivery_endpoint_id,
        "submitted_text": text,
        "conversation_id": send.snapshot.conversation_id,
        "stim_server_base_url": stim_server_base_url,
        "send": send,
    }))
}

#[derive(Debug, Default)]
struct AcceptancePayload {
    text: Option<String>,
    followup_text: Option<String>,
    target_endpoint_id: Option<String>,
    participant_id: Option<String>,
    delivery_endpoint_id: Option<String>,
}

impl AcceptancePayload {
    fn decode(payload: Value) -> Result<Self, String> {
        match payload {
            Value::Null => Ok(Self::default()),
            Value::String(text) => Ok(Self {
                text: normalized_string(text),
                ..Self::default()
            }),
            Value::Object(object) => Ok(Self {
                text: optional_string_field(&object, &["text"])?,
                followup_text: optional_string_field(
                    &object,
                    &["followup_text", "followupText", "followup"],
                )?,
                target_endpoint_id: optional_string_field(
                    &object,
                    &["target_endpoint_id", "targetEndpointId"],
                )?,
                participant_id: optional_string_field(
                    &object,
                    &["participant_id", "participantId"],
                )?,
                delivery_endpoint_id: optional_string_field(
                    &object,
                    &["delivery_endpoint_id", "deliveryEndpointId"],
                )?,
            }),
            other => Err(format!(
                "acceptance payload must be null, a JSON string, or a JSON object; got {other}"
            )),
        }
    }
}

fn optional_string_field(
    object: &Map<String, Value>,
    keys: &[&str],
) -> Result<Option<String>, String> {
    for key in keys {
        let Some(value) = object.get(*key) else {
            continue;
        };
        return match value {
            Value::Null => Ok(None),
            Value::String(text) => Ok(normalized_string(text.clone())),
            _ => Err(format!("payload field {key:?} must be a string")),
        };
    }

    Ok(None)
}

fn normalized_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}
