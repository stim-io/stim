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
    pub snapshot: Option<ControllerOperationSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerOperationStage {
    CommandAccepted,
    DeliveryStarted,
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
    pub last_user_text: Option<String>,
    pub last_assistant_text: Option<String>,
    pub final_sent_text: Option<String>,
    pub response_text_source: Option<String>,
    pub messages: Vec<ControllerOperationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOperationMessage {
    pub id: String,
    pub role: String,
    pub text: String,
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

#[cfg(test)]
mod tests {
    use super::{
        ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
        ControllerOperationStage, ControllerOperationStatus,
        CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
    };

    #[test]
    fn command_envelope_uses_operation_shaped_tags() {
        let command = ControllerOperationCommandEnvelope {
            schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
            operation_id: "op-1".into(),
            correlation_id: "corr-1".into(),
            command: ControllerOperationCommand::SendText {
                text: "hello".into(),
                target_endpoint_id: "endpoint-b".into(),
                conversation_id: None,
            },
        };

        let encoded = serde_json::to_value(&command).unwrap();

        assert_eq!(encoded["command"]["command"], "send-text");
        assert_eq!(encoded["schema_version"], 1);
    }

    #[test]
    fn terminal_events_are_named_by_stage() {
        let event = ControllerOperationEvent {
            schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
            event_id: "event-1".into(),
            operation_id: "op-1".into(),
            correlation_id: "corr-1".into(),
            causation_id: None,
            conversation_id: None,
            message_id: None,
            stage: ControllerOperationStage::OperationCompleted,
            status: ControllerOperationStatus::Completed,
            occurred_at: "2026-05-04T00:00:00Z".into(),
            detail: None,
            snapshot: None,
        };

        assert!(event.is_terminal());
    }
}
