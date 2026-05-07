use serde::{Deserialize, Serialize};
use serde_json::Value;
use stim_proto::MessageContent;

#[derive(Debug, Clone, Deserialize)]
pub struct FirstMessageRequest {
    pub text: String,
    pub target_endpoint_id: String,
    pub participant_id: Option<String>,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FirstMessageResponse {
    pub conversation_id: String,
    pub message_id: String,
    pub target_endpoint_id: String,
    pub participant_id: Option<String>,
    pub sent_text: String,
    pub final_sent_text: String,
    pub final_sent_content: MessageContentResponse,
    pub final_message_version: u64,
    pub response_text: String,
    pub response_content: MessageContentResponse,
    pub response_text_source: String,
    pub sent_envelope_id: String,
    pub response_envelope_id: String,
    pub receipt_result: String,
    pub receipt_detail: Option<String>,
    pub lifecycle_trace: Vec<LifecycleTraceResponse>,
    pub lifecycle_proof: LifecycleProofResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MessageContentResponse {
    pub parts: Vec<MessagePartResponse>,
    pub layout_hint: Option<LayoutHintResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConversationTranscriptResponse {
    pub conversation_id: String,
    pub messages: Vec<ConversationMessageResponse>,
    pub tool_activities: Vec<ConversationToolActivityResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConversationMessageResponse {
    pub id: String,
    pub role: String,
    pub author: String,
    pub sent_at_label: String,
    pub content: MessageContentResponse,
    pub delivery_state: Option<String>,
    pub meta_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationToolActivityResponse {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_call_seq: i64,
    pub result_state: String,
    pub tool_result_id: Option<String>,
    pub tool_result_seq: Option<i64>,
    pub exit_code: Option<i64>,
    pub duration_ms: Option<u64>,
    pub stdout_chars: Option<u64>,
    pub stderr_chars: Option<u64>,
    pub output_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessagePartResponse {
    Text { text: String },
    RawHtml { html: String },
    StimDomFragment { tree: Value },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutHintResponse {
    pub layout_family: Option<String>,
    pub min_height_px: Option<u32>,
    pub max_height_px: Option<u32>,
    pub vertical_pressure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleTraceResponse {
    pub operation: String,
    pub sent_envelope_id: String,
    pub ack_envelope_id: String,
    pub ack_message_id: String,
    pub ack_version: u64,
    pub response_text: String,
    pub response_text_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleProofResponse {
    pub create_ack_version: u64,
    pub patch_ack_version: u64,
    pub fix_ack_version: u64,
    pub final_message_version: u64,
    pub expected_final_text: String,
    pub controller_final_text: String,
    pub final_text_matches_expected: bool,
    pub version_progression_valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySnapshotResponse {
    pub endpoints: Vec<String>,
}

pub(crate) fn map_message_content(content: &MessageContent) -> MessageContentResponse {
    use stim_proto::{ContentPart, LayoutHint};

    fn map_layout_hint(layout_hint: &LayoutHint) -> LayoutHintResponse {
        LayoutHintResponse {
            layout_family: layout_hint.layout_family.clone(),
            min_height_px: layout_hint.min_height_px,
            max_height_px: layout_hint.max_height_px,
            vertical_pressure: layout_hint.vertical_pressure.clone(),
        }
    }

    MessageContentResponse {
        parts: content
            .parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text(text) => Some(MessagePartResponse::Text {
                    text: text.text.clone(),
                }),
                ContentPart::DomFragment(fragment) => match &fragment.payload {
                    stim_proto::DomFragmentPayload::StimDomFragmentV1 { tree, .. } => {
                        Some(MessagePartResponse::StimDomFragment { tree: tree.clone() })
                    }
                    stim_proto::DomFragmentPayload::RawHtml { html, .. } => {
                        Some(MessagePartResponse::RawHtml { html: html.clone() })
                    }
                },
                _ => None,
            })
            .collect(),
        layout_hint: content.layout_hint.as_ref().map(map_layout_hint),
    }
}
