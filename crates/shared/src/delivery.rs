use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use stim_proto::{
    DeliveryReceipt, DeliveryReceiptResult, DeliveryTarget, DiscoveryRecord, MessageEnvelope,
};

pub trait DeliveryPort {
    fn open_delivery_target(
        &self,
        discovery: &DiscoveryRecord,
    ) -> Result<DeliveryTarget, DeliveryError>;
    fn send_envelope(
        &self,
        target: &DeliveryTarget,
        envelope: MessageEnvelope,
    ) -> Result<DeliveryReceipt, DeliveryError>;
    fn receive_envelope(&self, listen_address: &str) -> Result<MessageEnvelope, DeliveryError>;
    fn close_delivery_target(&self, target: &DeliveryTarget) -> Result<(), DeliveryError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryError {
    UnsupportedCarrier,
    MissingAddress,
    MissingProtocolVersion,
    TargetNotOpen,
    ListenerNotBound,
    QueueEmpty,
    Poisoned,
    Remote(String),
}

#[derive(Default, Clone)]
pub struct LoopbackP2pCarrier {
    state: Arc<Mutex<CarrierState>>,
}

#[derive(Default)]
struct CarrierState {
    bound_addresses: HashMap<String, VecDeque<MessageEnvelope>>,
    open_targets: HashSet<String>,
}

impl LoopbackP2pCarrier {
    pub fn bind_listener(&self, listen_address: impl Into<String>) -> Result<(), DeliveryError> {
        let mut state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
        state
            .bound_addresses
            .entry(listen_address.into())
            .or_insert_with(VecDeque::new);
        Ok(())
    }
}

impl DeliveryPort for LoopbackP2pCarrier {
    fn open_delivery_target(
        &self,
        discovery: &DiscoveryRecord,
    ) -> Result<DeliveryTarget, DeliveryError> {
        if discovery.carrier_kind != "p2p" {
            return Err(DeliveryError::UnsupportedCarrier);
        }

        let selected_address = discovery
            .addresses
            .first()
            .cloned()
            .ok_or(DeliveryError::MissingAddress)?;
        let protocol_version = discovery
            .protocol_versions
            .first()
            .cloned()
            .ok_or(DeliveryError::MissingProtocolVersion)?;

        let target = DeliveryTarget {
            node_id: discovery.node_id.clone(),
            carrier_kind: discovery.carrier_kind.clone(),
            selected_address,
            protocol_version,
        };

        self.state
            .lock()
            .map_err(|_| DeliveryError::Poisoned)?
            .open_targets
            .insert(target.selected_address.clone());

        Ok(target)
    }

    fn send_envelope(
        &self,
        target: &DeliveryTarget,
        envelope: MessageEnvelope,
    ) -> Result<DeliveryReceipt, DeliveryError> {
        let mut state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
        if !state.open_targets.contains(&target.selected_address) {
            return Err(DeliveryError::TargetNotOpen);
        }

        let queue = state
            .bound_addresses
            .get_mut(&target.selected_address)
            .ok_or(DeliveryError::ListenerNotBound)?;
        let envelope_id = envelope.envelope_id.clone();
        queue.push_back(envelope);

        Ok(DeliveryReceipt {
            envelope_id,
            result: DeliveryReceiptResult::Accepted,
            detail: None,
        })
    }

    fn receive_envelope(&self, listen_address: &str) -> Result<MessageEnvelope, DeliveryError> {
        let mut state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
        let queue = state
            .bound_addresses
            .get_mut(listen_address)
            .ok_or(DeliveryError::ListenerNotBound)?;
        queue.pop_front().ok_or(DeliveryError::QueueEmpty)
    }

    fn close_delivery_target(&self, target: &DeliveryTarget) -> Result<(), DeliveryError> {
        self.state
            .lock()
            .map_err(|_| DeliveryError::Poisoned)?
            .open_targets
            .remove(&target.selected_address);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{DeliveryPort, LoopbackP2pCarrier};
    use stim_proto::{
        ContentPart, DiscoveryRecord, DomFragmentPart, DomFragmentPayload, EndpointDeclaration,
        MessageContent, MessageEnvelope, MessageOperation, MessageState, MutationPayload,
    };

    fn sample_discovery_record() -> DiscoveryRecord {
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

    #[test]
    fn opens_target_and_exchanges_envelope_over_loopback_p2p() {
        let carrier = LoopbackP2pCarrier::default();
        carrier.bind_listener("127.0.0.1:7001").unwrap();

        let target = carrier
            .open_delivery_target(&sample_discovery_record())
            .unwrap();
        let receipt = carrier.send_envelope(&target, sample_envelope()).unwrap();
        let received = carrier.receive_envelope("127.0.0.1:7001").unwrap();

        assert_eq!(receipt.result, stim_proto::DeliveryReceiptResult::Accepted);
        assert_eq!(receipt.envelope_id, "env-1");
        assert_eq!(received.envelope_id, "env-1");

        carrier.close_delivery_target(&target).unwrap();
    }
}
