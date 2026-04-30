use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use stim_proto::{DeliveryReceipt, DiscoveryRecord, MessageEnvelope, ProtocolSubmission};
use stim_shared::delivery::{DeliveryError, DeliveryPort};

use super::messages::{parse_acknowledgement, synthetic_response_envelope};

#[derive(Default)]
struct HttpSantiCarrierState {
    bound_addresses: HashMap<String, VecDeque<MessageEnvelope>>,
    open_targets: HashSet<String>,
}

#[derive(Clone)]
pub struct HttpSantiCarrier {
    response_listener_address: String,
    client: reqwest::blocking::Client,
    state: Arc<Mutex<HttpSantiCarrierState>>,
}

impl HttpSantiCarrier {
    pub fn new(response_listener_address: impl Into<String>) -> Self {
        Self {
            response_listener_address: response_listener_address.into(),
            client: reqwest::blocking::Client::new(),
            state: Arc::new(Mutex::new(HttpSantiCarrierState::default())),
        }
    }

    pub fn bind_listener(&self, listen_address: impl Into<String>) -> Result<(), DeliveryError> {
        let mut state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
        state
            .bound_addresses
            .entry(listen_address.into())
            .or_insert_with(VecDeque::new);
        Ok(())
    }

    fn enqueue_response(&self, envelope: MessageEnvelope) -> Result<(), DeliveryError> {
        let mut state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
        let queue = state
            .bound_addresses
            .get_mut(&self.response_listener_address)
            .ok_or(DeliveryError::ListenerNotBound)?;
        queue.push_back(envelope);
        Ok(())
    }
}

impl DeliveryPort for HttpSantiCarrier {
    fn open_delivery_target(
        &self,
        discovery: &DiscoveryRecord,
    ) -> Result<stim_proto::DeliveryTarget, DeliveryError> {
        if discovery.carrier_kind != "http" {
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

        let target = stim_proto::DeliveryTarget {
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
        target: &stim_proto::DeliveryTarget,
        envelope: MessageEnvelope,
    ) -> Result<DeliveryReceipt, DeliveryError> {
        {
            let state = self.state.lock().map_err(|_| DeliveryError::Poisoned)?;
            if !state.open_targets.contains(&target.selected_address) {
                return Err(DeliveryError::TargetNotOpen);
            }
        }

        let submission = self
            .client
            .post(format!("{}/api/v1/stim/envelopes", target.selected_address))
            .json(&envelope)
            .send()
            .map_err(|error| DeliveryError::Remote(format!("santi request failed: {error}")))?
            .error_for_status()
            .map_err(|error| DeliveryError::Remote(format!("santi status failed: {error}")))?
            .json::<ProtocolSubmission>()
            .map_err(|error| DeliveryError::Remote(format!("santi decode failed: {error}")))?;
        let parsed_acknowledgement = parse_acknowledgement(&submission.acknowledgement);

        self.enqueue_response(synthetic_response_envelope(
            &envelope,
            target,
            &parsed_acknowledgement,
            submission.reply.as_ref(),
        ))?;

        Ok(DeliveryReceipt {
            envelope_id: envelope.envelope_id,
            result: parsed_acknowledgement.receipt_result,
            detail: parsed_acknowledgement.receipt_detail,
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

    fn close_delivery_target(
        &self,
        target: &stim_proto::DeliveryTarget,
    ) -> Result<(), DeliveryError> {
        self.state
            .lock()
            .map_err(|_| DeliveryError::Poisoned)?
            .open_targets
            .remove(&target.selected_address);
        Ok(())
    }
}
