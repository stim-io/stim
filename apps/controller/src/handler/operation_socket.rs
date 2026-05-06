use std::{future::Future, pin::Pin};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

use crate::{
    model::ControllerHttpState,
    service::{
        command_decode_failed_event, run_load_transcript_operation, run_send_text_operation,
        unsupported_schema_event, OperationEventEmitter,
    },
};

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
                let _ = send_operation_event(&mut socket, command_decode_failed_event(error)).await;
                break;
            }
        };

        if command.schema_version != CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION {
            let _ = send_operation_event(&mut socket, unsupported_schema_event(&command)).await;
            break;
        }

        let mut emitter = SocketOperationEventEmitter {
            socket: &mut socket,
        };
        let result = match command.command.clone() {
            ControllerOperationCommand::SendText {
                text,
                target_endpoint_id,
                conversation_id,
            } => {
                run_send_text_operation(
                    state.clone(),
                    &command,
                    text,
                    target_endpoint_id,
                    conversation_id,
                    &mut emitter,
                )
                .await
            }
            ControllerOperationCommand::LoadTranscript { conversation_id } => {
                run_load_transcript_operation(
                    state.clone(),
                    &command,
                    conversation_id,
                    &mut emitter,
                )
                .await
            }
        };

        if result.is_err() {
            break;
        }
    }
}

struct SocketOperationEventEmitter<'a> {
    socket: &'a mut WebSocket,
}

impl OperationEventEmitter for SocketOperationEventEmitter<'_> {
    fn emit<'a>(
        &'a mut self,
        event: ControllerOperationEvent,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move { send_operation_event(self.socket, event).await })
    }
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
