use stim_shared::message_operation::{
    ControllerOperationReference, ControllerOperationReferenceKind,
};

use crate::{
    client::{ProductChatTurn, ProductChatTurnChunk, ProductChatTurnCompletion},
    model::ControllerProofSummary,
};

pub(super) fn summary_refs(
    summary: &ControllerProofSummary,
    product_turn: Option<&ProductChatTurn>,
    product_completion: Option<&ProductChatTurnCompletion>,
) -> Vec<ControllerOperationReference> {
    let mut references = vec![
        ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProtocolEnvelope,
            ledger_id: None,
            fact_id: None,
            message_id: Some(summary.message_id.clone()),
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: Some(summary.endpoint_id.clone()),
            envelope_id: Some(summary.envelope_id.clone()),
            reply_id: Some(summary.response_envelope_id.clone()),
            detail: Some("stim-proto envelope and reply handle observed by controller".into()),
        },
        projection_ref(&summary.conversation_id),
    ];
    if let Some(turn) = product_turn {
        references.extend(turn_refs(turn));
    }
    if let Some(completion) = product_completion {
        references.extend(completion_refs(completion));
    }
    references
}

pub(super) fn turn_refs(turn: &ProductChatTurn) -> Vec<ControllerOperationReference> {
    vec![
        ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
            ledger_id: Some("stim-server.product-chat".into()),
            fact_id: Some(turn.session_event_id.clone()),
            message_id: None,
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: None,
            envelope_id: None,
            reply_id: None,
            detail: Some(format!("product chat session {}", turn.session_id)),
        },
        ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
            ledger_id: Some("stim-server.product-chat".into()),
            fact_id: Some(turn.user_message_event_id.clone()),
            message_id: Some(turn.user_message_id.clone()),
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: None,
            envelope_id: None,
            reply_id: None,
            detail: Some("product user message created".into()),
        },
        ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
            ledger_id: Some("stim-server.product-chat".into()),
            fact_id: Some(turn.assistant_message_event_id.clone()),
            message_id: Some(turn.assistant_message_id.clone()),
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: None,
            envelope_id: None,
            reply_id: None,
            detail: Some("product assistant message created".into()),
        },
    ]
}

pub(super) fn completion_refs(
    completion: &ProductChatTurnCompletion,
) -> Vec<ControllerOperationReference> {
    let mut references = Vec::new();
    if let Some(chunk_event_id) = &completion.assistant_chunk_event_id {
        references.push(ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
            ledger_id: Some("stim-server.product-chat".into()),
            fact_id: Some(chunk_event_id.clone()),
            message_id: Some(completion.assistant_message_id.clone()),
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: None,
            envelope_id: None,
            reply_id: None,
            detail: Some("product assistant message chunk appended".into()),
        });
    }
    references.push(ControllerOperationReference {
        reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
        ledger_id: Some("stim-server.product-chat".into()),
        fact_id: Some(completion.assistant_final_event_id.clone()),
        message_id: Some(completion.assistant_message_id.clone()),
        content_id: None,
        revision_id: None,
        relation_id: None,
        participant_id: None,
        endpoint_id: None,
        envelope_id: None,
        reply_id: None,
        detail: Some("product assistant message finalized".into()),
    });
    references
}

pub(super) fn chunk_refs(chunk: &ProductChatTurnChunk) -> Vec<ControllerOperationReference> {
    vec![ControllerOperationReference {
        reference_kind: ControllerOperationReferenceKind::ProductMessageFact,
        ledger_id: Some("stim-server.product-chat".into()),
        fact_id: Some(chunk.assistant_chunk_event_id.clone()),
        message_id: Some(chunk.assistant_message_id.clone()),
        content_id: None,
        revision_id: None,
        relation_id: None,
        participant_id: None,
        endpoint_id: None,
        envelope_id: None,
        reply_id: None,
        detail: Some("product assistant message chunk appended".into()),
    }]
}

pub(super) fn projection_ref(conversation_id: &str) -> ControllerOperationReference {
    ControllerOperationReference {
        reference_kind: ControllerOperationReferenceKind::ControllerProjection,
        ledger_id: None,
        fact_id: None,
        message_id: None,
        content_id: None,
        revision_id: None,
        relation_id: None,
        participant_id: None,
        endpoint_id: None,
        envelope_id: None,
        reply_id: None,
        detail: Some(format!(
            "controller transcript projection for {conversation_id}"
        )),
    }
}
