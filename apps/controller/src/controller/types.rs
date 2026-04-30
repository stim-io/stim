use std::collections::HashMap;

use stim_proto::{DeliveryReceiptResult, DiscoveryRecord, MessageContent};
use stim_shared::delivery::DeliveryError;

pub(super) const DEFAULT_COMPOSE_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
pub(super) const DEFAULT_COMPOSE_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoundtripIds {
    pub(super) conversation_id: String,
    pub(super) message_id: String,
    pub(super) create_envelope_id: String,
    pub(super) patch_envelope_id: String,
    pub(super) fix_envelope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDiscoveryFixture {
    pub self_discovery: DiscoveryRecord,
    pub peer_discovery: DiscoveryRecord,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControllerProofSummary {
    pub server_base_url: String,
    pub endpoint_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub listen_address: String,
    pub envelope_id: String,
    pub final_sent_text: String,
    pub final_sent_content: MessageContent,
    pub final_message_version: u64,
    pub response_envelope_id: String,
    pub response_text: String,
    pub response_content: MessageContent,
    pub response_text_source: String,
    pub receipt_result: DeliveryReceiptResult,
    pub receipt_detail: Option<String>,
    pub lifecycle_trace: Vec<ControllerLifecycleStep>,
    pub lifecycle_proof: ControllerLifecycleProof,
    pub cached_endpoint_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerLifecycleStep {
    pub operation: String,
    pub sent_envelope_id: String,
    pub ack_envelope_id: String,
    pub ack_message_id: String,
    pub ack_version: u64,
    pub response_text: String,
    pub response_text_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerLifecycleProof {
    pub create_ack_version: u64,
    pub patch_ack_version: u64,
    pub fix_ack_version: u64,
    pub final_message_version: u64,
    pub expected_final_text: String,
    pub controller_final_text: String,
    pub final_text_matches_expected: bool,
    pub version_progression_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerState {
    server_base_url: String,
    cached_discovery: HashMap<String, DiscoveryRecord>,
}

impl ControllerState {
    pub(super) fn new(server_base_url: impl Into<String>) -> Self {
        Self {
            server_base_url: server_base_url.into(),
            cached_discovery: HashMap::new(),
        }
    }

    pub(super) fn cache_discovery_record(&mut self, discovery: DiscoveryRecord) {
        self.cached_discovery.insert(
            discovery.endpoint_declaration.endpoint_id.clone(),
            discovery,
        );
    }

    pub(super) fn cached_endpoint_count(&self) -> usize {
        self.cached_discovery.len()
    }

    pub(super) fn server_base_url(&self) -> &str {
        &self.server_base_url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerError {
    UnknownEndpoint(String),
    MissingAddress(String),
    UnsupportedPayload,
    Server(String),
    Delivery(DeliveryError),
}

impl From<DeliveryError> for ControllerError {
    fn from(value: DeliveryError) -> Self {
        Self::Delivery(value)
    }
}
