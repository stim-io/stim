use axum::{extract::State, http::StatusCode, Json};

use crate::{
    client::{
        complete_product_chat_turn, fail_product_chat_turn, resolve_delivery_endpoint,
        start_product_chat_turn, ProductChatTurnStart,
    },
    factory::sample_roundtrip_ids,
    model::{
        map_message_content, timestamp_now, ControllerError, ControllerHttpState,
        FirstMessageRequest, FirstMessageResponse, LifecycleProofResponse, LifecycleTraceResponse,
    },
    service,
};

pub(super) async fn first_message_roundtrip(
    State(state): State<ControllerHttpState>,
    Json(request): Json<FirstMessageRequest>,
) -> Result<Json<FirstMessageResponse>, (StatusCode, String)> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let target_endpoint_id = request.target_endpoint_id.clone();
    let participant_id = request.participant_id.clone();
    let text = request.text.clone();
    let conversation_id = request.conversation_id.clone();
    let self_discovery = state.self_discovery.clone();
    let (summary, ledger_warning) = tokio::task::spawn_blocking(move || {
        let delivery_endpoint_id = match normalized_participant_id(participant_id.as_deref()) {
            Some(participant_id) => {
                resolve_delivery_endpoint(&stim_server_base_url, participant_id)
                    .map_err(|(_, error)| ControllerError::Server(error))?
            }
            None => target_endpoint_id,
        };
        let include_bootstrap = conversation_id.is_none();
        let ids = sample_roundtrip_ids(conversation_id.as_deref());
        let user_participant_id = self_discovery.endpoint_declaration.endpoint_id.clone();
        let assistant_participant_id = normalized_participant_id(participant_id.as_deref())
            .map(str::to_string)
            .unwrap_or_else(|| delivery_endpoint_id.clone());
        let product_turn = start_product_chat_turn(
            &stim_server_base_url,
            ProductChatTurnStart {
                session_id: ids.conversation_id.clone(),
                user_message_id: ids.message_id.clone(),
                assistant_message_id: assistant_message_id(&ids.message_id),
                user_participant_id,
                assistant_participant_id,
                user_text: text.clone(),
                operation_id: format!("http-roundtrip-{}", ids.message_id),
                correlation_id: ids.create_envelope_id.clone(),
                causation_id: None,
            },
        )
        .map_err(|(_, error)| {
            ControllerError::Server(format!("product ledger start failed: {error}"))
        })?;
        match service::server_roundtrip_with_ids(
            &stim_server_base_url,
            &delivery_endpoint_id,
            &text,
            ids,
            include_bootstrap,
            self_discovery,
        ) {
            Ok(summary) => {
                let ledger_warning = complete_product_chat_turn(
                    &stim_server_base_url,
                    &product_turn,
                    &summary.response_text,
                    Some(&summary.response_envelope_id),
                )
                .err()
                .map(|(_, error)| format!("product ledger completion failed: {error}"));
                Ok((summary, ledger_warning))
            }
            Err(error) => {
                let failure_detail = format!("{error:?}");
                let _ = fail_product_chat_turn(
                    &stim_server_base_url,
                    &product_turn,
                    &failure_detail,
                    None,
                );
                Err(error)
            }
        }
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
        let mut roundtrip_detail = format!(
            "last roundtrip ok for endpoint {} envelope {}",
            summary.endpoint_id, summary.envelope_id
        );
        if let Some(ledger_warning) = ledger_warning {
            roundtrip_detail = format!("{roundtrip_detail} ; {ledger_warning}");
        }
        snapshot.detail = Some(match snapshot.detail.take() {
            Some(existing) if !existing.is_empty() => format!("{existing} ; {roundtrip_detail}"),
            _ => roundtrip_detail,
        });
    }

    Ok(Json(FirstMessageResponse {
        conversation_id: summary.conversation_id,
        message_id: summary.message_id,
        target_endpoint_id: summary.endpoint_id,
        participant_id: request.participant_id,
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

fn normalized_participant_id(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn assistant_message_id(user_message_id: &str) -> String {
    format!("{user_message_id}-assistant")
}
