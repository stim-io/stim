use stim_proto::{
    ContentPart, DiscoveryRecord, DomFragmentPart, DomFragmentPayload, EndpointDeclaration,
    MessageContent, MessageEnvelope, MessageOperation, MessageState, MutationPayload,
};
use stim_shared::delivery::{DeliveryPort, LoopbackP2pCarrier};

#[test]
fn exchanges_loopback_envelope() {
    let carrier = LoopbackP2pCarrier::default();
    carrier.bind_listener("127.0.0.1:7001").unwrap();

    let target = carrier.open_delivery_target(&sample_record()).unwrap();
    let receipt = carrier.send_envelope(&target, sample_envelope()).unwrap();
    let received = carrier.receive_envelope("127.0.0.1:7001").unwrap();

    assert_eq!(receipt.result, stim_proto::DeliveryReceiptResult::Accepted);
    assert_eq!(receipt.envelope_id, "env-1");
    assert_eq!(received.envelope_id, "env-1");

    carrier.close_delivery_target(&target).unwrap();
}

fn sample_record() -> DiscoveryRecord {
    DiscoveryRecord {
        node_id: "node-b".into(),
        endpoint_declaration: EndpointDeclaration {
            endpoint_id: "endpoint-b".into(),
            node_id: "node-b".into(),
            display_label: Some("peer-b".into()),
            endpoint_kind: Some("stim".into()),
            supported_protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
            supported_carriers: vec!["p2p".into()],
            content_capabilities: vec!["text".into(), "dom_fragment".into()],
            security_capabilities: vec!["sender_assertion".into()],
            declared_features: vec!["delivery".into()],
        },
        carrier_kind: "p2p".into(),
        addresses: vec!["127.0.0.1:7001".into()],
        protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
    }
}

fn sample_envelope() -> MessageEnvelope {
    MessageEnvelope {
        protocol_version: stim_proto::CURRENT_PROTOCOL_VERSION.into(),
        envelope_id: "env-1".into(),
        message_id: "msg-1".into(),
        conversation_id: "conv-1".into(),
        sender_node_id: "node-a".into(),
        sender_endpoint_id: "endpoint-a".into(),
        created_at: "2026-04-14T00:00:00Z".into(),
        session_bootstrap: None,
        sender_assertion: None,
        encryption_scope: None,
        recipient_key_refs: vec![],
        signature_ref: None,
        integrity_ref: None,
        state: MessageState::Pending,
        operation: MessageOperation::Create,
        base_version: None,
        new_version: 1,
        payload: MutationPayload::Create {
            content: MessageContent {
                parts: vec![ContentPart::DomFragment(DomFragmentPart {
                    part_id: "part-1".into(),
                    revision: 1,
                    metadata: None,
                    payload: DomFragmentPayload::RawHtml {
                        html: "<p>hello</p>".into(),
                        bindings: None,
                    },
                })],
                layout_hint: None,
            },
        },
    }
}
