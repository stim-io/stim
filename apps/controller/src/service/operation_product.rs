use crate::{
    client::{
        append_chat_turn_chunk, complete_product_chat_turn, fail_product_chat_turn,
        start_product_chat_turn, ProductChatTurn, ProductChatTurnChunk, ProductChatTurnCompletion,
        ProductChatTurnStart,
    },
    factory::RoundtripIds,
    model::ControllerHttpState,
};

use stim_shared::message_operation::ControllerOperationCommandEnvelope;

use super::operation_target::ResolvedTarget;

pub(super) async fn start_product_turn(
    state: ControllerHttpState,
    command: &ControllerOperationCommandEnvelope,
    ids: RoundtripIds,
    text: String,
    target: &ResolvedTarget,
    causation_id: Option<String>,
) -> Result<ProductChatTurn, String> {
    let base_url = state.stim_server_base_url.clone();
    let user_participant_id = state
        .self_discovery
        .endpoint_declaration
        .endpoint_id
        .clone();
    let assistant_participant_id = target.product_participant_id();
    let operation_id = command.operation_id.clone();
    let correlation_id = command.correlation_id.clone();
    tokio::task::spawn_blocking(move || {
        start_product_chat_turn(
            &base_url,
            ProductChatTurnStart {
                session_id: ids.conversation_id.clone(),
                user_message_id: ids.message_id.clone(),
                assistant_message_id: assistant_message_id(&ids.message_id),
                user_participant_id,
                assistant_participant_id,
                user_text: text,
                operation_id,
                correlation_id,
                causation_id,
            },
        )
    })
    .await
    .map_err(|error| format!("product ledger turn start join failed: {error}"))?
    .map_err(|(_, error)| format!("product ledger turn start failed: {error}"))
}

pub(super) async fn complete_product_turn(
    state: ControllerHttpState,
    turn: ProductChatTurn,
    response_text: String,
    causation_id: Option<String>,
) -> Result<ProductChatTurnCompletion, String> {
    let base_url = state.stim_server_base_url.clone();
    tokio::task::spawn_blocking(move || {
        complete_product_chat_turn(&base_url, &turn, &response_text, causation_id.as_deref())
    })
    .await
    .map_err(|error| format!("product ledger completion join failed: {error}"))?
    .map_err(|(_, error)| format!("product ledger completion failed: {error}"))
}

pub(super) async fn append_product_turn_chunk(
    state: ControllerHttpState,
    turn: ProductChatTurn,
    text: String,
    causation_id: Option<String>,
) -> Result<ProductChatTurnChunk, String> {
    let base_url = state.stim_server_base_url.clone();
    tokio::task::spawn_blocking(move || {
        append_chat_turn_chunk(&base_url, &turn, &text, causation_id.as_deref())
    })
    .await
    .map_err(|error| format!("product ledger chunk append join failed: {error}"))?
    .map_err(|(_, error)| format!("product ledger chunk append failed: {error}"))
}

pub(super) async fn fail_product_turn(
    state: ControllerHttpState,
    turn: ProductChatTurn,
    failure_detail: &str,
    causation_id: Option<String>,
) -> Result<ProductChatTurnCompletion, String> {
    let base_url = state.stim_server_base_url.clone();
    let failure_detail = failure_detail.to_string();
    tokio::task::spawn_blocking(move || {
        fail_product_chat_turn(&base_url, &turn, &failure_detail, causation_id.as_deref())
    })
    .await
    .map_err(|error| format!("product ledger failure finalize join failed: {error}"))?
    .map_err(|(_, error)| format!("product ledger failure finalize failed: {error}"))
}

pub(super) fn completion_response_text(
    response_text: &str,
    streamed_response_text: &str,
) -> String {
    if streamed_response_text.is_empty() {
        return response_text.to_string();
    }

    response_text
        .strip_prefix(streamed_response_text)
        .unwrap_or_default()
        .to_string()
}

fn assistant_message_id(user_message_id: &str) -> String {
    format!("{user_message_id}-assistant")
}
