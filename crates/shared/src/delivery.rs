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
