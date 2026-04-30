use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use stim_proto::{
    AcknowledgementResult, ContentPart, DeliveryReceiptResult, DomFragmentPart, DomFragmentPayload,
    LayoutHint, MessageContent, MessageEnvelope, MessageOperation, MessageState, MutationPayload,
    ProtocolAcknowledgement, ReplyHandle, TextPart,
};

use super::types::RoundtripIds;

static ROUNDTRIP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedAcknowledgement {
    pub(super) receipt_result: DeliveryReceiptResult,
    pub(super) receipt_detail: Option<String>,
    pub(super) ack_envelope_id: String,
    pub(super) ack_message_id: String,
    pub(super) ack_version: u64,
}

fn acknowledgement_to_receipt_result(
    acknowledgement: &ProtocolAcknowledgement,
) -> DeliveryReceiptResult {
    match acknowledgement.ack_result {
        AcknowledgementResult::Applied => DeliveryReceiptResult::Accepted,
        _ => DeliveryReceiptResult::Rejected,
    }
}

pub(super) fn parse_acknowledgement(
    acknowledgement: &ProtocolAcknowledgement,
) -> ParsedAcknowledgement {
    ParsedAcknowledgement {
        receipt_result: acknowledgement_to_receipt_result(acknowledgement),
        receipt_detail: acknowledgement.detail.clone(),
        ack_envelope_id: acknowledgement.ack_envelope_id.clone(),
        ack_message_id: acknowledgement.ack_message_id.clone(),
        ack_version: acknowledgement.ack_version,
    }
}

pub(super) fn user_text_content(text: &str) -> MessageContent {
    MessageContent {
        parts: vec![ContentPart::Text(TextPart {
            part_id: "user-text".into(),
            revision: 1,
            metadata: None,
            text: text.into(),
        })],
        layout_hint: Some(LayoutHint {
            layout_family: Some("bubble".into()),
            min_height_px: None,
            max_height_px: None,
            vertical_pressure: Some("compact".into()),
            metadata: None,
        }),
    }
}

pub(super) fn assistant_card_content(text: &str) -> MessageContent {
    MessageContent {
        parts: vec![ContentPart::DomFragment(DomFragmentPart {
            part_id: "assistant-card".into(),
            revision: 1,
            metadata: None,
            payload: DomFragmentPayload::StimDomFragmentV1 {
                tree: serde_json::json!({
                    "tag": "section",
                    "props": {
                        "data-stim-role": "assistant-card"
                    },
                    "children": [
                        {
                            "tag": "p",
                            "children": [
                                {
                                    "text": text,
                                }
                            ]
                        }
                    ]
                }),
                bindings: None,
            },
        })],
        layout_hint: Some(LayoutHint {
            layout_family: Some("card".into()),
            min_height_px: Some(112),
            max_height_px: None,
            vertical_pressure: Some("expand".into()),
            metadata: None,
        }),
    }
}

pub(super) fn synthetic_response_envelope(
    request: &MessageEnvelope,
    target: &stim_proto::DeliveryTarget,
    acknowledgement: &ParsedAcknowledgement,
    reply_handle: Option<&ReplyHandle>,
) -> MessageEnvelope {
    sample_text_envelope(
        &acknowledgement.ack_envelope_id,
        &acknowledgement.ack_message_id,
        &request.conversation_id,
        &target.node_id,
        "endpoint-b",
        &lifecycle_response_preview(request),
        MessageState::Fixed,
        MessageOperation::Create,
        None,
        acknowledgement.ack_version,
        None,
        None,
        Some(json!({
            "response_text_source": "protocol_ack",
            "reply_id": reply_handle.map(|reply| reply.reply_id.clone()),
        })),
    )
}

fn lifecycle_response_preview(request: &MessageEnvelope) -> String {
    match &request.payload {
        MutationPayload::Create { content } => content
            .parts
            .first()
            .and_then(|part| match part {
                ContentPart::Text(TextPart { text, .. }) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "create applied".into()),
        MutationPayload::Patch { patches } => patches
            .first()
            .and_then(|patch| patch.merge.get("text"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "patch applied".into()),
        MutationPayload::Fix {} => "message fixed".into(),
        MutationPayload::Insert { .. } => "insert applied".into(),
        MutationPayload::Remove { .. } => "remove applied".into(),
    }
}

pub(super) fn sample_roundtrip_ids(conversation_id: Option<&str>) -> RoundtripIds {
    let sequence = ROUNDTRIP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let unique = format!("{millis}-{sequence}");

    RoundtripIds {
        conversation_id: conversation_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("conv-{unique}")),
        message_id: format!("msg-{unique}"),
        create_envelope_id: format!("env-{unique}-create"),
        patch_envelope_id: format!("env-{unique}-patch"),
        fix_envelope_id: format!("env-{unique}-fix"),
    }
}

pub(super) fn sample_create_envelope(
    ids: &RoundtripIds,
    text: &str,
    include_bootstrap: bool,
) -> MessageEnvelope {
    sample_text_envelope(
        &ids.create_envelope_id,
        &ids.message_id,
        &ids.conversation_id,
        "node-a",
        "endpoint-a",
        text,
        MessageState::Pending,
        MessageOperation::Create,
        None,
        1,
        include_bootstrap.then_some(stim_proto::SessionBootstrap {
            participants: vec!["endpoint-a".into(), "endpoint-b".into()],
            created_by: "endpoint-a".into(),
            created_at: "2026-04-14T00:00:00Z".into(),
        }),
        None,
        None,
    )
}

pub(super) fn sample_patch_envelope(
    ids: &RoundtripIds,
    base_version: u64,
    text: &str,
) -> MessageEnvelope {
    sample_text_envelope(
        &ids.patch_envelope_id,
        &ids.message_id,
        &ids.conversation_id,
        "node-a",
        "endpoint-a",
        text,
        MessageState::Pending,
        MessageOperation::Patch,
        Some(base_version),
        base_version + 1,
        None,
        Some(json!({ "text": text })),
        None,
    )
}

pub(super) fn sample_fix_envelope(ids: &RoundtripIds, base_version: u64) -> MessageEnvelope {
    sample_text_envelope(
        &ids.fix_envelope_id,
        &ids.message_id,
        &ids.conversation_id,
        "node-a",
        "endpoint-a",
        "",
        MessageState::Fixed,
        MessageOperation::Fix,
        Some(base_version),
        base_version + 1,
        None,
        None,
        None,
    )
}

fn sample_text_envelope(
    envelope_id: &str,
    message_id: &str,
    conversation_id: &str,
    sender_node_id: &str,
    sender_endpoint_id: &str,
    text: &str,
    state: MessageState,
    operation: MessageOperation,
    base_version: Option<u64>,
    new_version: u64,
    session_bootstrap: Option<stim_proto::SessionBootstrap>,
    patch_merge: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
) -> MessageEnvelope {
    MessageEnvelope {
        protocol_version: stim_proto::CURRENT_PROTOCOL_VERSION.into(),
        envelope_id: envelope_id.into(),
        message_id: message_id.into(),
        conversation_id: conversation_id.into(),
        sender_node_id: sender_node_id.into(),
        sender_endpoint_id: sender_endpoint_id.into(),
        created_at: "2026-04-14T00:00:00Z".into(),
        session_bootstrap,
        sender_assertion: None,
        encryption_scope: None,
        recipient_key_refs: vec![],
        signature_ref: None,
        integrity_ref: None,
        state,
        operation: operation.clone(),
        base_version,
        new_version,
        payload: match operation {
            MessageOperation::Create => MutationPayload::Create {
                content: MessageContent {
                    parts: vec![ContentPart::Text(TextPart {
                        part_id: "part-1".into(),
                        revision: 1,
                        metadata,
                        text: text.into(),
                    })],
                    layout_hint: None,
                },
            },
            MessageOperation::Patch => MutationPayload::Patch {
                patches: vec![stim_proto::PatchOperation {
                    index: 0,
                    merge: patch_merge.unwrap_or_else(|| json!({ "text": text })),
                }],
            },
            MessageOperation::Fix => MutationPayload::Fix {},
            MessageOperation::Insert => MutationPayload::Insert { items: vec![] },
            MessageOperation::Remove => MutationPayload::Remove { indexes: vec![] },
        },
    }
}
