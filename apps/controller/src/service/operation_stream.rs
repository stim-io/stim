use stim_shared::message_operation::{
    ControllerOperationCommandEnvelope, ControllerOperationMessageDelta, ControllerOperationStage,
    ControllerOperationStatus,
};

use crate::{
    client::ProductChatTurn,
    factory::RoundtripIds,
    model::{timestamp_now, ControllerError, ControllerHttpState, ControllerProofSummary},
};

use super::{
    operation_event, operation_product::append_product_turn_chunk, operation_refs::chunk_refs,
    server_roundtrip_with_stream, OperationEventEmitter, OperationEventPayload,
};

pub(super) async fn send_text_product_stream<E>(
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    input: ProductStreamInput,
    emitter: &mut E,
    causation_id: &mut Option<String>,
) -> Result<(ControllerProofSummary, String), String>
where
    E: OperationEventEmitter,
{
    let ProductStreamInput {
        target_endpoint_id,
        text,
        ids,
        include_bootstrap,
        product_turn,
    } = input;
    let (reply_delta_sender, mut reply_delta_receiver) =
        tokio::sync::mpsc::unbounded_channel::<String>();
    let mut roundtrip_task = Box::pin(send_text_roundtrip(
        state.clone(),
        target_endpoint_id,
        text,
        ids,
        include_bootstrap,
        reply_delta_sender,
    ));
    let mut streamed_response_text = String::new();
    let mut reply_delta_stream_open = true;
    let mut stream_error = None;

    let summary_result = loop {
        tokio::select! {
            result = &mut roundtrip_task => break result,
            maybe_delta = reply_delta_receiver.recv(), if reply_delta_stream_open => {
                let Some(delta) = maybe_delta else {
                    reply_delta_stream_open = false;
                    continue;
                };
                if delta.is_empty() || stream_error.is_some() {
                    continue;
                }

                match append_product_turn_chunk(
                    state.clone(),
                    product_turn.clone(),
                    delta.clone(),
                    causation_id.clone(),
                )
                .await
                {
                    Ok(chunk) => {
                        streamed_response_text.push_str(&delta);
                        *causation_id = Some(
                            emitter
                                .emit(operation_event(
                                    command,
                                    ControllerOperationStage::MessageChunkAppended,
                                    ControllerOperationStatus::Running,
                                    OperationEventPayload {
                                        causation_id: causation_id.clone(),
                                        conversation_id: Some(product_turn.session_id.clone()),
                                        message_id: Some(chunk.assistant_message_id.clone()),
                                        detail: Some("assistant message chunk appended".into()),
                                        references: chunk_refs(&chunk),
                                        message_delta: Some(ControllerOperationMessageDelta {
                                            message_id: chunk.assistant_message_id,
                                            role: "assistant".into(),
                                            text: delta,
                                        }),
                                        snapshot: None,
                                    },
                                ))
                                .await?,
                        );
                    }
                    Err(error) => {
                        stream_error = Some(error);
                    }
                }
            }
        }
    };

    if let Some(error) = stream_error {
        return Err(error);
    }

    summary_result.map(|summary| (summary, streamed_response_text))
}

pub(super) struct ProductStreamInput {
    pub(super) target_endpoint_id: String,
    pub(super) text: String,
    pub(super) ids: RoundtripIds,
    pub(super) include_bootstrap: bool,
    pub(super) product_turn: ProductChatTurn,
}

async fn send_text_roundtrip(
    state: ControllerHttpState,
    target_endpoint_id: String,
    text: String,
    ids: RoundtripIds,
    include_bootstrap: bool,
    reply_delta_sender: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<ControllerProofSummary, String> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let self_discovery = state.self_discovery.clone();
    let summary = tokio::task::spawn_blocking(move || {
        server_roundtrip_with_stream(
            &stim_server_base_url,
            &target_endpoint_id,
            &text,
            ids,
            include_bootstrap,
            self_discovery,
            move |delta| {
                reply_delta_sender.send(delta.to_string()).map_err(|error| {
                    ControllerError::Server(format!(
                        "controller reply delta dispatch failed: {error}"
                    ))
                })
            },
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
