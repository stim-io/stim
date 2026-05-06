use serde::Deserialize;

use crate::model::{
    ConversationMessageResponse, ConversationToolActivityResponse, ConversationTranscriptResponse,
    MessageContentResponse, MessagePartResponse,
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SantiSessionMessagesResponse {
    pub(crate) messages: Vec<SantiSessionMessageResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SantiSessionToolActivitiesResponse {
    pub(crate) tool_activities: Vec<SantiSessionToolActivityResponse>,
}

impl SantiSessionToolActivitiesResponse {
    pub(crate) fn empty() -> Self {
        Self {
            tool_activities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SantiSessionMessageResponse {
    pub(crate) id: String,
    pub(crate) actor_type: String,
    pub(crate) actor_id: String,
    pub(crate) session_seq: i64,
    pub(crate) content_text: String,
    pub(crate) state: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SantiSessionToolActivityResponse {
    pub(crate) tool_call_id: String,
    pub(crate) tool_name: String,
    pub(crate) tool_call_seq: i64,
    pub(crate) result_state: String,
    pub(crate) tool_result_id: Option<String>,
    pub(crate) tool_result_seq: Option<i64>,
    pub(crate) exit_code: Option<i64>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) stdout_chars: Option<u64>,
    pub(crate) stderr_chars: Option<u64>,
    pub(crate) output_summary: Option<String>,
}

pub(crate) fn map_santi_transcript(
    conversation_id: String,
    messages: SantiSessionMessagesResponse,
    tool_activities: SantiSessionToolActivitiesResponse,
) -> ConversationTranscriptResponse {
    ConversationTranscriptResponse {
        conversation_id,
        messages: messages
            .messages
            .into_iter()
            .map(map_santi_message)
            .collect(),
        tool_activities: tool_activities
            .tool_activities
            .into_iter()
            .map(map_santi_tool_activity)
            .collect(),
    }
}

fn map_santi_message(message: SantiSessionMessageResponse) -> ConversationMessageResponse {
    let role = match message.actor_type.as_str() {
        "account" => "user",
        "soul" => "assistant",
        _ => "system",
    };
    let author = match role {
        "user" => "You".to_string(),
        "assistant" => "stim".to_string(),
        _ => message.actor_id.clone(),
    };

    ConversationMessageResponse {
        id: message.id,
        role: role.to_string(),
        author,
        sent_at_label: format!("#{}", message.session_seq),
        content: MessageContentResponse {
            parts: vec![MessagePartResponse::Text {
                text: message.content_text,
            }],
            layout_hint: None,
        },
        delivery_state: (role == "user").then(|| "sent".to_string()),
        meta_label: Some(format!("{} · {}", message.state, message.created_at)),
    }
}

fn map_santi_tool_activity(
    tool_activity: SantiSessionToolActivityResponse,
) -> ConversationToolActivityResponse {
    ConversationToolActivityResponse {
        tool_call_id: tool_activity.tool_call_id,
        tool_name: tool_activity.tool_name,
        tool_call_seq: tool_activity.tool_call_seq,
        result_state: tool_activity.result_state,
        tool_result_id: tool_activity.tool_result_id,
        tool_result_seq: tool_activity.tool_result_seq,
        exit_code: tool_activity.exit_code,
        duration_ms: tool_activity.duration_ms,
        stdout_chars: tool_activity.stdout_chars,
        stderr_chars: tool_activity.stderr_chars,
        output_summary: tool_activity.output_summary,
    }
}
