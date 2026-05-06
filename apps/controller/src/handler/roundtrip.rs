use axum::{extract::State, http::StatusCode, Json};

use crate::{
    model::{
        map_message_content, timestamp_now, ControllerHttpState, FirstMessageRequest,
        FirstMessageResponse, LifecycleProofResponse, LifecycleTraceResponse,
    },
    service,
};

pub(super) async fn first_message_roundtrip(
    State(state): State<ControllerHttpState>,
    Json(request): Json<FirstMessageRequest>,
) -> Result<Json<FirstMessageResponse>, (StatusCode, String)> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let target_endpoint_id = request.target_endpoint_id.clone();
    let text = request.text.clone();
    let conversation_id = request.conversation_id.clone();
    let self_discovery = state.self_discovery.clone();
    let summary = tokio::task::spawn_blocking(move || {
        service::message_roundtrip_via_server(
            &stim_server_base_url,
            &target_endpoint_id,
            &text,
            conversation_id.as_deref(),
            self_discovery,
        )
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller blocking roundtrip join failed: {error}"),
        )
    })?
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")))?;

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

    Ok(Json(FirstMessageResponse {
        conversation_id: summary.conversation_id,
        message_id: summary.message_id,
        target_endpoint_id: request.target_endpoint_id,
        sent_text: request.text,
        final_sent_text: summary.final_sent_text,
        final_sent_content: map_message_content(&summary.final_sent_content),
        final_message_version: summary.final_message_version,
        response_text: summary.response_text,
        response_content: map_message_content(&summary.response_content),
        response_text_source: summary.response_text_source,
        sent_envelope_id: summary.envelope_id,
        response_envelope_id: summary.response_envelope_id,
        receipt_result: format!("{:?}", summary.receipt_result).to_lowercase(),
        receipt_detail: summary.receipt_detail,
        lifecycle_trace: summary
            .lifecycle_trace
            .into_iter()
            .map(|step| LifecycleTraceResponse {
                operation: step.operation,
                sent_envelope_id: step.sent_envelope_id,
                ack_envelope_id: step.ack_envelope_id,
                ack_message_id: step.ack_message_id,
                ack_version: step.ack_version,
                response_text: step.response_text,
                response_text_source: step.response_text_source,
            })
            .collect(),
        lifecycle_proof: LifecycleProofResponse {
            create_ack_version: summary.lifecycle_proof.create_ack_version,
            patch_ack_version: summary.lifecycle_proof.patch_ack_version,
            fix_ack_version: summary.lifecycle_proof.fix_ack_version,
            final_message_version: summary.lifecycle_proof.final_message_version,
            expected_final_text: summary.lifecycle_proof.expected_final_text,
            controller_final_text: summary.lifecycle_proof.controller_final_text,
            final_text_matches_expected: summary.lifecycle_proof.final_text_matches_expected,
            version_progression_valid: summary.lifecycle_proof.version_progression_valid,
        },
    }))
}
