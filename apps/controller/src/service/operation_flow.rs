use stim_shared::message_operation::{
    ControllerOperationCommandEnvelope, ControllerOperationStage, ControllerOperationStatus,
};

use crate::model::{timestamp_now, ControllerHttpState, ControllerProofSummary};

use super::{
    load_operation_snapshot, message_roundtrip_via_server, operation_event, OperationEventEmitter,
    OperationEventPayload,
};

pub(crate) async fn run_send_text_operation<E>(
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    text: String,
    target_endpoint_id: String,
    conversation_id: Option<String>,
    emitter: &mut E,
) -> Result<(), String>
where
    E: OperationEventEmitter,
{
    let mut causation_id = Some(
        emitter
            .emit(operation_event(
                command,
                ControllerOperationStage::CommandAccepted,
                ControllerOperationStatus::Accepted,
                OperationEventPayload {
                    causation_id: None,
                    conversation_id: conversation_id.clone(),
                    message_id: None,
                    detail: Some("controller accepted send-text command".into()),
                    snapshot: None,
                },
            ))
            .await?,
    );

    causation_id = Some(
        emitter
            .emit(operation_event(
                command,
                ControllerOperationStage::DeliveryStarted,
                ControllerOperationStatus::Running,
                OperationEventPayload {
                    causation_id,
                    conversation_id: conversation_id.clone(),
                    message_id: None,
                    detail: Some(format!("sending text to {target_endpoint_id}")),
                    snapshot: None,
                },
            ))
            .await?,
    );

    match send_text_roundtrip(state.clone(), &target_endpoint_id, &text, conversation_id).await {
        Ok(summary) => {
            let conversation_id = summary.conversation_id.clone();
            let message_id = summary.message_id.clone();
            causation_id = Some(
                emitter
                    .emit(operation_event(
                        command,
                        ControllerOperationStage::ConversationSelected,
                        ControllerOperationStatus::Completed,
                        OperationEventPayload {
                            causation_id,
                            conversation_id: Some(conversation_id.clone()),
                            message_id: Some(message_id.clone()),
                            detail: Some("controller selected conversation".into()),
                            snapshot: None,
                        },
                    ))
                    .await?,
            );
            causation_id = Some(
                emitter
                    .emit(operation_event(
                        command,
                        ControllerOperationStage::DeliveryCompleted,
                        ControllerOperationStatus::Completed,
                        OperationEventPayload {
                            causation_id,
                            conversation_id: Some(conversation_id.clone()),
                            message_id: Some(message_id.clone()),
                            detail: Some(format!(
                                "roundtrip completed with {} response source",
                                summary.response_text_source
                            )),
                            snapshot: None,
                        },
                    ))
                    .await?,
            );

            let snapshot = load_operation_snapshot(
                state,
                conversation_id.clone(),
                Some(summary.final_sent_text),
                Some(summary.response_text_source),
            )
            .await?;
            causation_id = Some(
                emitter
                    .emit(operation_event(
                        command,
                        ControllerOperationStage::TranscriptLoaded,
                        ControllerOperationStatus::Completed,
                        OperationEventPayload {
                            causation_id,
                            conversation_id: Some(conversation_id.clone()),
                            message_id: Some(message_id.clone()),
                            detail: Some("controller loaded persisted transcript snapshot".into()),
                            snapshot: Some(snapshot.clone()),
                        },
                    ))
                    .await?,
            );
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationCompleted,
                    ControllerOperationStatus::Completed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: Some(conversation_id),
                        message_id: Some(message_id),
                        detail: Some("send-text operation completed".into()),
                        snapshot: Some(snapshot),
                    },
                ))
                .await?;
        }
        Err(error) => {
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationFailed,
                    ControllerOperationStatus::Failed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: None,
                        message_id: None,
                        detail: Some(error),
                        snapshot: None,
                    },
                ))
                .await?;
        }
    }

    Ok(())
}

pub(crate) async fn run_load_transcript_operation<E>(
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    conversation_id: String,
    emitter: &mut E,
) -> Result<(), String>
where
    E: OperationEventEmitter,
{
    let mut causation_id = Some(
        emitter
            .emit(operation_event(
                command,
                ControllerOperationStage::CommandAccepted,
                ControllerOperationStatus::Accepted,
                OperationEventPayload {
                    causation_id: None,
                    conversation_id: Some(conversation_id.clone()),
                    message_id: None,
                    detail: Some("controller accepted load-transcript command".into()),
                    snapshot: None,
                },
            ))
            .await?,
    );

    match load_operation_snapshot(state, conversation_id.clone(), None, None).await {
        Ok(snapshot) => {
            causation_id = Some(
                emitter
                    .emit(operation_event(
                        command,
                        ControllerOperationStage::TranscriptLoaded,
                        ControllerOperationStatus::Completed,
                        OperationEventPayload {
                            causation_id,
                            conversation_id: Some(conversation_id.clone()),
                            message_id: None,
                            detail: Some("controller loaded persisted transcript snapshot".into()),
                            snapshot: Some(snapshot.clone()),
                        },
                    ))
                    .await?,
            );
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationCompleted,
                    ControllerOperationStatus::Completed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: Some(conversation_id),
                        message_id: None,
                        detail: Some("load-transcript operation completed".into()),
                        snapshot: Some(snapshot),
                    },
                ))
                .await?;
        }
        Err(error) => {
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationFailed,
                    ControllerOperationStatus::Failed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: Some(conversation_id),
                        message_id: None,
                        detail: Some(error),
                        snapshot: None,
                    },
                ))
                .await?;
        }
    }

    Ok(())
}

async fn send_text_roundtrip(
    state: ControllerHttpState,
    target_endpoint_id: &str,
    text: &str,
    conversation_id: Option<String>,
) -> Result<ControllerProofSummary, String> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let target_endpoint_id = target_endpoint_id.to_string();
    let text = text.to_string();
    let self_discovery = state.self_discovery.clone();
    let summary = tokio::task::spawn_blocking(move || {
        message_roundtrip_via_server(
            &stim_server_base_url,
            &target_endpoint_id,
            &text,
            conversation_id.as_deref(),
            self_discovery,
        )
    })
    .await
    .map_err(|error| format!("controller blocking roundtrip join failed: {error}"))?
    .map_err(|error| format!("controller roundtrip failed: {error:?}"))?;

    if let Ok(mut snapshot) = state.snapshot.lock() {
        snapshot.published_at = timestamp_now();
        let roundtrip_detail = format!(
            "last roundtrip ok for endpoint {} envelope {}",
            summary.endpoint_id, summary.envelope_id
        );
        snapshot.detail = Some(match snapshot.detail.take() {
            Some(existing) if !existing.is_empty() => format!("{existing} ; {roundtrip_detail}"),
            _ => roundtrip_detail,
        });
    }

    Ok(summary)
}
