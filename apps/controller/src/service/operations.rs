use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationMessage, ControllerOperationSnapshot, ControllerOperationStage,
    ControllerOperationStatus, ControllerOperationToolActivity,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

use crate::controller;

use super::{
    clock::timestamp_now,
    transcript::{fetch_santi_conversation_messages, fetch_santi_conversation_tool_activities},
    types::{
        map_santi_transcript, ControllerHttpState, ConversationToolActivityResponse,
        ConversationTranscriptResponse, MessageContentResponse, MessagePartResponse,
    },
};

static CONTROLLER_OPERATION_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) async fn controller_operation_socket(
    State(state): State<ControllerHttpState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_controller_operation_socket(state, socket))
}

async fn handle_controller_operation_socket(state: ControllerHttpState, mut socket: WebSocket) {
    while let Some(message) = socket.recv().await {
        let text = match message {
            Ok(Message::Text(text)) => text.to_string(),
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(_) => break,
        };

        let command = match serde_json::from_str::<ControllerOperationCommandEnvelope>(&text) {
            Ok(command) => command,
            Err(error) => {
                let command = fallback_failed_command();
                let _ = send_operation_event(
                    &mut socket,
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
                            snapshot: None,
                        },
                    ),
                )
                .await;
                break;
            }
        };

        if command.schema_version != CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION {
            let _ = send_operation_event(
                &mut socket,
                operation_event(
                    &command,
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
                        snapshot: None,
                    },
                ),
            )
            .await;
            break;
        }

        let result = match command.command.clone() {
            ControllerOperationCommand::SendText {
                text,
                target_endpoint_id,
                conversation_id,
            } => {
                handle_send_text_operation(
                    &mut socket,
                    state.clone(),
                    &command,
                    text,
                    target_endpoint_id,
                    conversation_id,
                )
                .await
            }
            ControllerOperationCommand::LoadTranscript { conversation_id } => {
                handle_load_transcript_operation(
                    &mut socket,
                    state.clone(),
                    &command,
                    conversation_id,
                )
                .await
            }
        };

        if result.is_err() {
            break;
        }
    }
}

async fn handle_send_text_operation(
    socket: &mut WebSocket,
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    text: String,
    target_endpoint_id: String,
    conversation_id: Option<String>,
) -> Result<(), String> {
    let mut causation_id = Some(
        send_operation_event(
            socket,
            operation_event(
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
            ),
        )
        .await?,
    );

    causation_id = Some(
        send_operation_event(
            socket,
            operation_event(
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
            ),
        )
        .await?,
    );

    match send_text_roundtrip(state.clone(), &target_endpoint_id, &text, conversation_id).await {
        Ok(summary) => {
            let conversation_id = summary.conversation_id.clone();
            let message_id = summary.message_id.clone();
            causation_id = Some(
                send_operation_event(
                    socket,
                    operation_event(
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
                    ),
                )
                .await?,
            );
            causation_id = Some(
                send_operation_event(
                    socket,
                    operation_event(
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
                    ),
                )
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
                send_operation_event(
                    socket,
                    operation_event(
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
                    ),
                )
                .await?,
            );
            send_operation_event(
                socket,
                operation_event(
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
                ),
            )
            .await?;
        }
        Err(error) => {
            send_operation_event(
                socket,
                operation_event(
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
                ),
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_load_transcript_operation(
    socket: &mut WebSocket,
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    conversation_id: String,
) -> Result<(), String> {
    let mut causation_id = Some(
        send_operation_event(
            socket,
            operation_event(
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
            ),
        )
        .await?,
    );

    match load_operation_snapshot(state, conversation_id.clone(), None, None).await {
        Ok(snapshot) => {
            causation_id = Some(
                send_operation_event(
                    socket,
                    operation_event(
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
                    ),
                )
                .await?,
            );
            send_operation_event(
                socket,
                operation_event(
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
                ),
            )
            .await?;
        }
        Err(error) => {
            send_operation_event(
                socket,
                operation_event(
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
                ),
            )
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
) -> Result<crate::controller::ControllerProofSummary, String> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let target_endpoint_id = target_endpoint_id.to_string();
    let text = text.to_string();
    let self_discovery = state.self_discovery.clone();
    let summary = tokio::task::spawn_blocking(move || {
        controller::message_roundtrip_via_server(
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

async fn load_operation_snapshot(
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
        fetch_santi_conversation_tool_activities(&santi_base_url, &conversation_id_for_fetch)
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

async fn send_operation_event(
    socket: &mut WebSocket,
    event: ControllerOperationEvent,
) -> Result<String, String> {
    let event_id = event.event_id.clone();
    let body = serde_json::to_string(&event)
        .map_err(|error| format!("failed to serialize controller operation event: {error}"))?;
    socket
        .send(Message::Text(body.into()))
        .await
        .map_err(|error| format!("failed to send controller operation event: {error}"))?;
    Ok(event_id)
}

struct OperationEventPayload {
    causation_id: Option<String>,
    conversation_id: Option<String>,
    message_id: Option<String>,
    detail: Option<String>,
    snapshot: Option<ControllerOperationSnapshot>,
}

fn operation_event(
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
        snapshot: payload.snapshot,
    }
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
