use serde::{Deserialize, Serialize};

pub const CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationCommandEnvelope {
    pub schema_version: u32,
    pub operation_id: String,
    pub correlation_id: String,
    pub command: ControllerOperationCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum ControllerOperationCommand {
    SendText {
        text: String,
        target_endpoint_id: String,
        participant_id: Option<String>,
        conversation_id: Option<String>,
    },
    LoadTranscript {
        conversation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationEvent {
    pub schema_version: u32,
    pub event_id: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub stage: ControllerOperationStage,
    pub status: ControllerOperationStatus,
    pub occurred_at: String,
    pub detail: Option<String>,
    #[serde(default)]
    pub references: Vec<ControllerOperationReference>,
    pub message_delta: Option<ControllerOperationMessageDelta>,
    pub snapshot: Option<ControllerOperationSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationReference {
    pub reference_kind: ControllerOperationReferenceKind,
    pub ledger_id: Option<String>,
    pub fact_id: Option<String>,
    pub message_id: Option<String>,
    pub content_id: Option<String>,
    pub revision_id: Option<String>,
    pub relation_id: Option<String>,
    pub participant_id: Option<String>,
    pub endpoint_id: Option<String>,
    pub envelope_id: Option<String>,
    pub reply_id: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerOperationReferenceKind {
    ProductMessageFact,
    SantiImFact,
    SantiRuntimeFact,
    ProtocolEnvelope,
    Participant,
    DeliveryEndpoint,
    ControllerProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerOperationStage {
    CommandAccepted,
    DeliveryTargetResolved,
    DeliveryStarted,
    MessageChunkAppended,
    ConversationSelected,
    DeliveryCompleted,
    TranscriptLoaded,
    OperationCompleted,
    OperationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerOperationStatus {
    Accepted,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationSnapshot {
    pub conversation_id: String,
    pub message_count: usize,
    pub user_message_count: usize,
    pub assistant_message_count: usize,
    pub tool_activity_count: usize,
    pub tool_result_count: usize,
    pub last_user_text: Option<String>,
    pub last_assistant_text: Option<String>,
    pub final_sent_text: Option<String>,
    pub response_text_source: Option<String>,
    pub messages: Vec<ControllerOperationMessage>,
    pub tool_activities: Vec<ControllerOperationToolActivity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationMessage {
    pub id: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationMessageDelta {
    pub message_id: String,
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationToolActivity {
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

impl ControllerOperationEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.stage,
            ControllerOperationStage::OperationCompleted
                | ControllerOperationStage::OperationFailed
        )
    }
}
