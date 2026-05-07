use stim_shared::message_operation::{
    ControllerOperationMessage, ControllerOperationSnapshot, ControllerOperationToolActivity,
};

use crate::{
    client::{fetch_santi_conversation_messages, fetch_santi_tool_activity, map_santi_transcript},
    model::{
        ControllerHttpState, ConversationToolActivityResponse, ConversationTranscriptResponse,
        MessageContentResponse, MessagePartResponse,
    },
};

pub(crate) async fn load_operation_snapshot(
    state: ControllerHttpState,
    conversation_id: String,
    final_sent_text: Option<String>,
    response_text_source: Option<String>,
) -> Result<ControllerOperationSnapshot, String> {
    let santi_base_url = state.santi_base_url.clone();
    let conversation_id_for_fetch = conversation_id.clone();
    let messages = tokio::task::spawn_blocking(move || {
        fetch_santi_conversation_messages(&santi_base_url, &conversation_id_for_fetch)
    })
    .await
    .map_err(|error| format!("controller transcript fetch join failed: {error}"))?
    .map_err(|error| format!("controller transcript fetch failed: {error}"))?;

    let santi_base_url = state.santi_base_url.clone();
    let conversation_id_for_fetch = conversation_id.clone();
    let tool_activities = tokio::task::spawn_blocking(move || {
        fetch_santi_tool_activity(&santi_base_url, &conversation_id_for_fetch)
    })
    .await
    .map_err(|error| format!("controller tool activity fetch join failed: {error}"))?
    .map_err(|error| format!("controller tool activity fetch failed: {error}"))?;

    let transcript =
        map_santi_transcript(conversation_id, messages.payload, tool_activities.payload);
    Ok(operation_snapshot(
        transcript,
        final_sent_text,
        response_text_source,
    ))
}

fn operation_snapshot(
    transcript: ConversationTranscriptResponse,
    final_sent_text: Option<String>,
    response_text_source: Option<String>,
) -> ControllerOperationSnapshot {
    let messages = transcript
        .messages
        .iter()
        .map(|message| ControllerOperationMessage {
            id: message.id.clone(),
            role: message.role.clone(),
            text: first_text(&message.content).unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let last_user_text = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.text.clone());
    let last_assistant_text = messages
        .iter()
        .rev()
        .find(|message| message.role == "assistant")
        .map(|message| message.text.clone());
    let tool_activities = transcript
        .tool_activities
        .iter()
        .map(map_tool_activity)
        .collect::<Vec<_>>();

    ControllerOperationSnapshot {
        conversation_id: transcript.conversation_id,
        message_count: messages.len(),
        user_message_count: messages
            .iter()
            .filter(|message| message.role == "user")
            .count(),
        assistant_message_count: messages
            .iter()
            .filter(|message| message.role == "assistant")
            .count(),
        tool_activity_count: tool_activities.len(),
        tool_result_count: tool_activities
            .iter()
            .filter(|activity| activity.tool_result_id.is_some())
            .count(),
        last_user_text,
        last_assistant_text,
        final_sent_text,
        response_text_source,
        messages,
        tool_activities,
    }
}

fn map_tool_activity(
    activity: &ConversationToolActivityResponse,
) -> ControllerOperationToolActivity {
    ControllerOperationToolActivity {
        tool_call_id: activity.tool_call_id.clone(),
        tool_name: activity.tool_name.clone(),
        tool_call_seq: activity.tool_call_seq,
        result_state: activity.result_state.clone(),
        tool_result_id: activity.tool_result_id.clone(),
        tool_result_seq: activity.tool_result_seq,
        exit_code: activity.exit_code,
        duration_ms: activity.duration_ms,
        stdout_chars: activity.stdout_chars,
        stderr_chars: activity.stderr_chars,
        output_summary: activity.output_summary.clone(),
    }
}

fn first_text(content: &MessageContentResponse) -> Option<String> {
    content.parts.iter().find_map(|part| match part {
        MessagePartResponse::Text { text } => Some(text.clone()),
        _ => None,
    })
}
