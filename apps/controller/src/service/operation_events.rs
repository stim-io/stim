use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationMessageDelta, ControllerOperationReference, ControllerOperationSnapshot,
    ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

use crate::model::timestamp_now;

static CONTROLLER_OPERATION_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) trait OperationEventEmitter {
    fn emit<'a>(
        &'a mut self,
        event: ControllerOperationEvent,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

pub(crate) struct OperationEventPayload {
    pub(crate) causation_id: Option<String>,
    pub(crate) conversation_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) references: Vec<ControllerOperationReference>,
    pub(crate) message_delta: Option<ControllerOperationMessageDelta>,
    pub(crate) snapshot: Option<ControllerOperationSnapshot>,
}

pub(crate) fn operation_event(
    command: &ControllerOperationCommandEnvelope,
    stage: ControllerOperationStage,
    status: ControllerOperationStatus,
    payload: OperationEventPayload,
) -> ControllerOperationEvent {
    ControllerOperationEvent {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        event_id: next_operation_event_id(),
        operation_id: command.operation_id.clone(),
        correlation_id: command.correlation_id.clone(),
        causation_id: payload.causation_id,
        conversation_id: payload.conversation_id,
        message_id: payload.message_id,
        stage,
        status,
        occurred_at: timestamp_now(),
        detail: payload.detail,
        references: payload.references,
        message_delta: payload.message_delta,
        snapshot: payload.snapshot,
    }
}

pub(crate) fn command_decode_failed_event(
    error: impl std::fmt::Display,
) -> ControllerOperationEvent {
    let command = fallback_failed_command();
    operation_event(
        &command,
        ControllerOperationStage::OperationFailed,
        ControllerOperationStatus::Failed,
        OperationEventPayload {
            causation_id: None,
            conversation_id: None,
            message_id: None,
            detail: Some(format!(
                "controller operation command decode failed: {error}"
            )),
            references: vec![],
            message_delta: None,
            snapshot: None,
        },
    )
}

pub(crate) fn unsupported_schema_event(
    command: &ControllerOperationCommandEnvelope,
) -> ControllerOperationEvent {
    operation_event(
        command,
        ControllerOperationStage::OperationFailed,
        ControllerOperationStatus::Failed,
        OperationEventPayload {
            causation_id: None,
            conversation_id: None,
            message_id: None,
            detail: Some(format!(
                "unsupported controller operation schema_version {}",
                command.schema_version
            )),
            references: vec![],
            message_delta: None,
            snapshot: None,
        },
    )
}

fn next_operation_event_id() -> String {
    let sequence = CONTROLLER_OPERATION_EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("controller-event-{sequence}")
}

fn fallback_failed_command() -> ControllerOperationCommandEnvelope {
    ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: "invalid-command".into(),
        correlation_id: "invalid-command".into(),
        command: ControllerOperationCommand::LoadTranscript {
            conversation_id: "unknown".into(),
        },
    }
}
