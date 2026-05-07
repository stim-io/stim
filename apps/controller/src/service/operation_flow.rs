use stim_shared::message_operation::{
    ControllerOperationCommandEnvelope, ControllerOperationStage, ControllerOperationStatus,
};

use crate::{factory::sample_roundtrip_ids, model::ControllerHttpState};

use super::{
    load_operation_snapshot, operation_event,
    operation_product::{
        complete_product_turn, completion_response_text, fail_product_turn, start_product_turn,
    },
    operation_refs::{projection_ref, summary_refs, turn_refs},
    operation_stream::{send_text_product_stream, ProductStreamInput},
    operation_target::resolve_target,
    OperationEventEmitter, OperationEventPayload,
};

pub(crate) async fn run_send_text_operation<E>(
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    text: String,
    target_endpoint_id: String,
    participant_id: Option<String>,
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
                    references: vec![],
                    message_delta: None,
                    snapshot: None,
                },
            ))
            .await?,
    );

    let resolved_delivery_target = match resolve_target(
        state.stim_server_base_url.clone(),
        target_endpoint_id,
        participant_id,
    )
    .await
    {
        Ok(target) => target,
        Err(error) => {
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationFailed,
                    ControllerOperationStatus::Failed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: conversation_id.clone(),
                        message_id: None,
                        detail: Some(error),
                        references: vec![],
                        message_delta: None,
                        snapshot: None,
                    },
                ))
                .await?;
            return Ok(());
        }
    };
    causation_id = Some(
        emitter
            .emit(operation_event(
                command,
                ControllerOperationStage::DeliveryTargetResolved,
                ControllerOperationStatus::Completed,
                OperationEventPayload {
                    causation_id,
                    conversation_id: conversation_id.clone(),
                    message_id: None,
                    detail: Some(resolved_delivery_target.detail.clone()),
                    references: resolved_delivery_target.references(),
                    message_delta: None,
                    snapshot: None,
                },
            ))
            .await?,
    );

    let include_bootstrap = conversation_id.is_none();
    let ids = sample_roundtrip_ids(conversation_id.as_deref());
    let product_turn = match start_product_turn(
        state.clone(),
        command,
        ids.clone(),
        text.clone(),
        &resolved_delivery_target,
        causation_id.clone(),
    )
    .await
    {
        Ok(turn) => turn,
        Err(error) => {
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationFailed,
                    ControllerOperationStatus::Failed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: Some(ids.conversation_id.clone()),
                        message_id: Some(ids.message_id.clone()),
                        detail: Some(error),
                        references: resolved_delivery_target.references(),
                        message_delta: None,
                        snapshot: None,
                    },
                ))
                .await?;
            return Ok(());
        }
    };

    let mut delivery_started_references = resolved_delivery_target.endpoint_reference();
    delivery_started_references.extend(turn_refs(&product_turn));
    causation_id = Some(
        emitter
            .emit(operation_event(
                command,
                ControllerOperationStage::DeliveryStarted,
                ControllerOperationStatus::Running,
                OperationEventPayload {
                    causation_id,
                    conversation_id: Some(ids.conversation_id.clone()),
                    message_id: Some(ids.message_id.clone()),
                    detail: Some(format!(
                        "sending text to {}",
                        resolved_delivery_target.endpoint_id
                    )),
                    references: delivery_started_references,
                    message_delta: None,
                    snapshot: None,
                },
            ))
            .await?,
    );

    match send_text_product_stream(
        state.clone(),
        command,
        ProductStreamInput {
            target_endpoint_id: resolved_delivery_target.endpoint_id,
            text,
            ids,
            include_bootstrap,
            product_turn: product_turn.clone(),
        },
        emitter,
        &mut causation_id,
    )
    .await
    {
        Ok((summary, streamed_response_text)) => {
            let conversation_id = summary.conversation_id.clone();
            let message_id = summary.message_id.clone();
            let product_completion = match complete_product_turn(
                state.clone(),
                product_turn.clone(),
                completion_response_text(&summary.response_text, &streamed_response_text),
                causation_id.clone(),
            )
            .await
            {
                Ok(completion) => Some(completion),
                Err(error) => {
                    if let Ok(mut snapshot) = state.snapshot.lock() {
                        snapshot.detail = Some(match snapshot.detail.take() {
                            Some(existing) if !existing.is_empty() => {
                                format!("{existing} ; {error}")
                            }
                            _ => error,
                        });
                    }
                    None
                }
            };
            let summary_references =
                summary_refs(&summary, Some(&product_turn), product_completion.as_ref());
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
                            references: summary_references.clone(),
                            message_delta: None,
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
                            references: summary_references.clone(),
                            message_delta: None,
                            snapshot: None,
                        },
                    ))
                    .await?,
            );

            let snapshot = load_operation_snapshot(
                state,
                conversation_id.clone(),
                Some(summary.final_sent_text.clone()),
                Some(summary.response_text_source.clone()),
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
                            references: summary_references.clone(),
                            message_delta: None,
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
                        references: summary_references,
                        message_delta: None,
                        snapshot: Some(snapshot),
                    },
                ))
                .await?;
        }
        Err(error) => {
            let mut failure_detail = error;
            if let Err(ledger_error) = fail_product_turn(
                state.clone(),
                product_turn.clone(),
                &failure_detail,
                causation_id.clone(),
            )
            .await
            {
                failure_detail = format!("{failure_detail} ; {ledger_error}");
            }
            emitter
                .emit(operation_event(
                    command,
                    ControllerOperationStage::OperationFailed,
                    ControllerOperationStatus::Failed,
                    OperationEventPayload {
                        causation_id,
                        conversation_id: Some(product_turn.session_id.clone()),
                        message_id: Some(product_turn.assistant_message_id.clone()),
                        detail: Some(failure_detail),
                        references: turn_refs(&product_turn),
                        message_delta: None,
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
                    references: vec![projection_ref(&conversation_id)],
                    message_delta: None,
                    snapshot: None,
                },
            ))
            .await?,
    );

    match load_operation_snapshot(state, conversation_id.clone(), None, None).await {
        Ok(snapshot) => {
            let projection_reference = projection_ref(&conversation_id);
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
                            references: vec![projection_reference.clone()],
                            message_delta: None,
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
                        references: vec![projection_reference],
                        message_delta: None,
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
                        references: vec![],
                        message_delta: None,
                        snapshot: None,
                    },
                ))
                .await?;
        }
    }

    Ok(())
}
