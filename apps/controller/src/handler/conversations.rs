use axum::{extract::State, http::StatusCode, Json};

use crate::{
    client::{
        fetch_santi_conversation_messages, fetch_santi_conversation_tool_activities,
        map_santi_transcript,
    },
    fetch::FetchError,
    model::{ControllerHttpState, ConversationTranscriptResponse},
};

pub(super) async fn conversation_messages(
    State(state): State<ControllerHttpState>,
    axum::extract::Path(conversation_id): axum::extract::Path<String>,
) -> Result<Json<ConversationTranscriptResponse>, (StatusCode, String)> {
    let santi_base_url = state.santi_base_url.clone();
    let conversation_id_for_fetch = conversation_id.clone();
    let messages = tokio::task::spawn_blocking(move || {
        fetch_santi_conversation_messages(&santi_base_url, &conversation_id_for_fetch)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller transcript fetch join failed: {error}"),
        )
    })?
    .map_err(map_santi_fetch_error)?;

    let santi_base_url = state.santi_base_url.clone();
    let conversation_id_for_fetch = conversation_id.clone();
    let tool_activities = tokio::task::spawn_blocking(move || {
        fetch_santi_conversation_tool_activities(&santi_base_url, &conversation_id_for_fetch)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller tool activity fetch join failed: {error}"),
        )
    })?
    .map_err(map_santi_fetch_error)?;

    Ok(Json(map_santi_transcript(
        conversation_id,
        messages.payload,
        tool_activities.payload,
    )))
}

fn map_santi_fetch_error(error: FetchError) -> (StatusCode, String) {
    let status = match error.metadata.last_status {
        Some(404) => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_GATEWAY,
    };

    (status, error.to_string())
}
