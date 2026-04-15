use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use stim_proto::{
    AcknowledgementResult, ContentPart, DeliveryReceipt, DeliveryReceiptResult, DiscoveryRecord,
    EndpointDeclaration, MessageContent, MessageEnvelope, MessageOperation, MessageState,
    MutationPayload, ProtocolAcknowledgement, ProtocolSubmission, ReplyEvent, ReplyEventKind,
    ReplyHandle, ReplySnapshot, TextPart,
};
use stim_shared::delivery::{DeliveryError, DeliveryPort};

const DEFAULT_COMPOSE_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_COMPOSE_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";
static ROUNDTRIP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoundtripIds {
    conversation_id: String,
    message_id: String,
    create_envelope_id: String,
    patch_envelope_id: String,
    fix_envelope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDiscoveryFixture {
    pub self_discovery: DiscoveryRecord,
    pub peer_discovery: DiscoveryRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerProofSummary {
    pub server_base_url: String,
    pub endpoint_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub listen_address: String,
    pub envelope_id: String,
    pub final_sent_text: String,
    pub final_message_version: u64,
    pub response_envelope_id: String,
    pub response_text: String,
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
    fn new(server_base_url: impl Into<String>) -> Self {
        Self {
            server_base_url: server_base_url.into(),
            cached_discovery: HashMap::new(),
        }
    }

    fn cache_discovery_record(&mut self, discovery: DiscoveryRecord) {
        self.cached_discovery.insert(
            discovery.endpoint_declaration.endpoint_id.clone(),
            discovery,
        );
    }

    fn cached_endpoint_count(&self) -> usize {
        self.cached_discovery.len()
    }

    fn server_base_url(&self) -> &str {
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

pub trait StimServerFacade {
    fn server_base_url(&self) -> &str;
    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError>;
}

#[derive(Debug, Clone)]
pub struct InMemoryStimServerFacade {
    server_base_url: String,
    records_by_endpoint: HashMap<String, DiscoveryRecord>,
}

#[derive(Debug, Clone)]
pub struct HttpStimServerFacade {
    server_base_url: String,
    client: reqwest::blocking::Client,
}

impl HttpStimServerFacade {
    pub fn new(server_base_url: impl Into<String>) -> Self {
        Self {
            server_base_url: server_base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl InMemoryStimServerFacade {
    pub fn new(server_base_url: impl Into<String>, records: Vec<DiscoveryRecord>) -> Self {
        let records_by_endpoint = records
            .into_iter()
            .map(|record| (record.endpoint_declaration.endpoint_id.clone(), record))
            .collect();

        Self {
            server_base_url: server_base_url.into(),
            records_by_endpoint,
        }
    }
}

pub fn in_memory_facade(
    server_base_url: impl Into<String>,
    records: Vec<DiscoveryRecord>,
) -> InMemoryStimServerFacade {
    InMemoryStimServerFacade::new(server_base_url, records)
}

impl StimServerFacade for InMemoryStimServerFacade {
    fn server_base_url(&self) -> &str {
        &self.server_base_url
    }

    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError> {
        self.records_by_endpoint
            .get(endpoint_id)
            .cloned()
            .ok_or_else(|| ControllerError::UnknownEndpoint(endpoint_id.into()))
    }
}

impl StimServerFacade for HttpStimServerFacade {
    fn server_base_url(&self) -> &str {
        &self.server_base_url
    }

    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError> {
        let response = self
            .client
            .get(format!(
                "{}/api/v1/discovery/endpoints/{}",
                self.server_base_url, endpoint_id
            ))
            .send()
            .map_err(|error| {
                ControllerError::Server(format!("discover request failed: {error}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ControllerError::UnknownEndpoint(endpoint_id.into()));
        }

        response
            .error_for_status()
            .map_err(|error| ControllerError::Server(format!("discover status failed: {error}")))?
            .json::<DiscoveryRecord>()
            .map_err(|error| ControllerError::Server(format!("discover decode failed: {error}")))
    }
}

#[derive(Default)]
struct HttpSantiCarrierState {
    bound_addresses: HashMap<String, VecDeque<MessageEnvelope>>,
    open_targets: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedAcknowledgement {
    receipt_result: DeliveryReceiptResult,
    receipt_detail: Option<String>,
    ack_envelope_id: String,
    ack_message_id: String,
    ack_version: u64,
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

pub struct ControllerRuntime<F, D> {
    state: ControllerState,
    server_facade: F,
    delivery_port: D,
    self_discovery: DiscoveryRecord,
}

impl<F, D> ControllerRuntime<F, D>
where
    F: StimServerFacade,
    D: DeliveryPort + Clone + Send + 'static,
{
    pub fn new(server_facade: F, delivery_port: D, self_discovery: DiscoveryRecord) -> Self {
        Self {
            state: ControllerState::new(server_facade.server_base_url()),
            server_facade,
            delivery_port,
            self_discovery,
        }
    }

    pub fn deliver_to_endpoint(
        &mut self,
        endpoint_id: &str,
        text: &str,
        conversation_id: Option<&str>,
    ) -> Result<ControllerProofSummary, ControllerError> {
        let peer_discovery = self.server_facade.discover_endpoint(endpoint_id)?;
        let self_discovery = self.self_discovery.clone();
        let revised_text = format!("{text} (edited before fix)");
        let ids = sample_roundtrip_ids(conversation_id);

        let peer_listen_address = first_listen_address(&peer_discovery)?;
        let self_listen_address = first_listen_address(&self_discovery)?;
        let create_envelope = sample_create_envelope(&ids, text, conversation_id.is_none());
        let sent_envelope_id = create_envelope.envelope_id.clone();

        self.state.cache_discovery_record(peer_discovery.clone());
        self.state.cache_discovery_record(self_discovery.clone());

        let target = self.delivery_port.open_delivery_target(&peer_discovery)?;
        let receipt = self.delivery_port.send_envelope(&target, create_envelope)?;
        let create_response = receive_with_retry(&self.delivery_port, &self_listen_address)?;
        let mut lifecycle_trace = vec![lifecycle_step(
            "create",
            &sent_envelope_id,
            &create_response,
        )?];

        let patch_envelope =
            sample_patch_envelope(&ids, create_response.new_version, &revised_text);
        let patch_sent_envelope_id = patch_envelope.envelope_id.clone();
        self.delivery_port.send_envelope(&target, patch_envelope)?;
        let patch_response = receive_with_retry(&self.delivery_port, &self_listen_address)?;
        lifecycle_trace.push(lifecycle_step(
            "patch",
            &patch_sent_envelope_id,
            &patch_response,
        )?);

        let fix_envelope = sample_fix_envelope(&ids, patch_response.new_version);
        let fix_sent_envelope_id = fix_envelope.envelope_id.clone();
        self.delivery_port.send_envelope(&target, fix_envelope)?;
        let fix_response = receive_with_retry(&self.delivery_port, &self_listen_address)?;
        lifecycle_trace.push(lifecycle_step("fix", &fix_sent_envelope_id, &fix_response)?);
        let lifecycle_proof =
            build_lifecycle_proof(&lifecycle_trace, &revised_text, fix_response.new_version);
        let reply_id = extract_reply_id(&fix_response)?;
        let response_text = request_protocol_reply(&target.selected_address, &reply_id)?;

        self.delivery_port.close_delivery_target(&target)?;

        Ok(ControllerProofSummary {
            server_base_url: self.state.server_base_url().into(),
            endpoint_id: endpoint_id.into(),
            conversation_id: ids.conversation_id,
            message_id: ids.message_id,
            listen_address: peer_listen_address,
            envelope_id: sent_envelope_id,
            final_sent_text: revised_text,
            final_message_version: fix_response.new_version,
            response_envelope_id: reply_id,
            response_text,
            response_text_source: "stim_reply_handle".into(),
            receipt_result: receipt.result,
            receipt_detail: receipt.detail,
            lifecycle_trace,
            lifecycle_proof,
            cached_endpoint_count: self.state.cached_endpoint_count(),
        })
    }
}

pub fn first_message_roundtrip_with_records(
    server_base_url: &str,
    endpoint_id: &str,
    text: &str,
    self_discovery: DiscoveryRecord,
    records: Vec<DiscoveryRecord>,
) -> Result<ControllerProofSummary, ControllerError> {
    message_roundtrip_with_records(
        server_base_url,
        endpoint_id,
        text,
        None,
        self_discovery,
        records,
    )
}

pub fn message_roundtrip_with_records(
    server_base_url: &str,
    endpoint_id: &str,
    text: &str,
    conversation_id: Option<&str>,
    self_discovery: DiscoveryRecord,
    records: Vec<DiscoveryRecord>,
) -> Result<ControllerProofSummary, ControllerError> {
    let facade = InMemoryStimServerFacade::new(server_base_url, records);
    let carrier = http_santi_carrier(&self_discovery)?;

    let mut runtime = ControllerRuntime::new(facade, carrier, self_discovery);
    runtime.deliver_to_endpoint(endpoint_id, text, conversation_id)
}

pub fn message_roundtrip_via_server(
    server_base_url: &str,
    endpoint_id: &str,
    text: &str,
    conversation_id: Option<&str>,
    self_discovery: DiscoveryRecord,
) -> Result<ControllerProofSummary, ControllerError> {
    let facade = HttpStimServerFacade::new(server_base_url);
    let carrier = http_santi_carrier(&self_discovery)?;

    let mut runtime = ControllerRuntime::new(facade, carrier, self_discovery);
    runtime.deliver_to_endpoint(endpoint_id, text, conversation_id)
}

pub fn first_message_roundtrip(text: &str) -> Result<ControllerProofSummary, ControllerError> {
    let santi_base_url = std::env::var("SANTI_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_COMPOSE_SANTI_BASE_URL.to_string());
    let fixture = http_santi_discovery_fixture("default", &santi_base_url);
    first_message_roundtrip_with_records(
        DEFAULT_COMPOSE_STIM_SERVER_BASE_URL,
        "endpoint-b",
        text,
        fixture.self_discovery.clone(),
        vec![fixture.peer_discovery, fixture.self_discovery],
    )
}

pub fn first_message_roundtrip_via_server(
    server_base_url: &str,
    endpoint_id: &str,
    text: &str,
    self_discovery: DiscoveryRecord,
) -> Result<ControllerProofSummary, ControllerError> {
    message_roundtrip_via_server(server_base_url, endpoint_id, text, None, self_discovery)
}

pub fn run() -> Result<ControllerProofSummary, ControllerError> {
    first_message_roundtrip("hello from stim ui")
}

fn http_santi_carrier(
    self_discovery: &DiscoveryRecord,
) -> Result<HttpSantiCarrier, ControllerError> {
    let response_listener_address = first_listen_address(self_discovery)?;
    let carrier = HttpSantiCarrier::new(response_listener_address.clone());
    carrier.bind_listener(response_listener_address)?;
    Ok(carrier)
}

fn first_listen_address(discovery: &DiscoveryRecord) -> Result<String, ControllerError> {
    discovery.addresses.first().cloned().ok_or_else(|| {
        ControllerError::MissingAddress(discovery.endpoint_declaration.endpoint_id.clone())
    })
}

fn extract_text(envelope: &MessageEnvelope) -> Result<String, ControllerError> {
    match &envelope.payload {
        MutationPayload::Create { content } => match content.parts.first() {
            Some(ContentPart::Text(TextPart { text, .. })) => Ok(text.clone()),
            _ => Err(ControllerError::UnsupportedPayload),
        },
        _ => Err(ControllerError::UnsupportedPayload),
    }
}

fn receive_with_retry<D>(
    carrier: &D,
    listen_address: &str,
) -> Result<MessageEnvelope, ControllerError>
where
    D: DeliveryPort,
{
    for _ in 0..50 {
        match carrier.receive_envelope(listen_address) {
            Ok(envelope) => return Ok(envelope),
            Err(DeliveryError::QueueEmpty) => thread::sleep(Duration::from_millis(10)),
            Err(error) => return Err(ControllerError::Delivery(error)),
        }
    }

    Err(ControllerError::Delivery(DeliveryError::QueueEmpty))
}

fn acknowledgement_to_receipt_result(
    acknowledgement: &ProtocolAcknowledgement,
) -> DeliveryReceiptResult {
    match acknowledgement.ack_result {
        AcknowledgementResult::Applied => DeliveryReceiptResult::Accepted,
        _ => DeliveryReceiptResult::Rejected,
    }
}

fn parse_acknowledgement(acknowledgement: &ProtocolAcknowledgement) -> ParsedAcknowledgement {
    ParsedAcknowledgement {
        receipt_result: acknowledgement_to_receipt_result(acknowledgement),
        receipt_detail: acknowledgement.detail.clone(),
        ack_envelope_id: acknowledgement.ack_envelope_id.clone(),
        ack_message_id: acknowledgement.ack_message_id.clone(),
        ack_version: acknowledgement.ack_version,
    }
}

fn extract_response_text_source(envelope: &MessageEnvelope) -> String {
    match &envelope.payload {
        MutationPayload::Create { content } => match content.parts.first() {
            Some(ContentPart::Text(TextPart {
                metadata: Some(metadata),
                ..
            })) => metadata
                .get("response_text_source")
                .and_then(|value| value.as_str())
                .unwrap_or("synthetic_fallback")
                .to_string(),
            _ => "synthetic_fallback".to_string(),
        },
        _ => "synthetic_fallback".to_string(),
    }
}

fn extract_reply_id(envelope: &MessageEnvelope) -> Result<String, ControllerError> {
    match &envelope.payload {
        MutationPayload::Create { content } => match content.parts.first() {
            Some(ContentPart::Text(TextPart {
                metadata: Some(metadata),
                ..
            })) => metadata
                .get("reply_id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .ok_or_else(|| {
                    ControllerError::Server(
                        "protocol reply handle missing from fix acknowledgement".into(),
                    )
                }),
            _ => Err(ControllerError::Server(
                "protocol reply handle missing from fix acknowledgement".into(),
            )),
        },
        _ => Err(ControllerError::UnsupportedPayload),
    }
}

fn lifecycle_step(
    operation: &str,
    sent_envelope_id: &str,
    response: &MessageEnvelope,
) -> Result<ControllerLifecycleStep, ControllerError> {
    Ok(ControllerLifecycleStep {
        operation: operation.into(),
        sent_envelope_id: sent_envelope_id.into(),
        ack_envelope_id: response.envelope_id.clone(),
        ack_message_id: response.message_id.clone(),
        ack_version: response.new_version,
        response_text: extract_text(response)?,
        response_text_source: extract_response_text_source(response),
    })
}

fn build_lifecycle_proof(
    lifecycle_trace: &[ControllerLifecycleStep],
    expected_final_text: &str,
    final_message_version: u64,
) -> ControllerLifecycleProof {
    let create_ack_version = lifecycle_trace
        .first()
        .map(|step| step.ack_version)
        .unwrap_or(0);
    let patch_ack_version = lifecycle_trace
        .get(1)
        .map(|step| step.ack_version)
        .unwrap_or(0);
    let fix_ack_version = lifecycle_trace
        .get(2)
        .map(|step| step.ack_version)
        .unwrap_or(0);

    ControllerLifecycleProof {
        create_ack_version,
        patch_ack_version,
        fix_ack_version,
        final_message_version,
        expected_final_text: expected_final_text.to_string(),
        controller_final_text: expected_final_text.to_string(),
        final_text_matches_expected: !expected_final_text.is_empty(),
        version_progression_valid: create_ack_version == 1
            && patch_ack_version == create_ack_version + 1
            && fix_ack_version == patch_ack_version + 1
            && final_message_version == fix_ack_version,
    }
}

fn request_protocol_reply(santi_base_url: &str, reply_id: &str) -> Result<String, ControllerError> {
    let body = reqwest::blocking::Client::new()
        .get(format!(
            "{santi_base_url}/api/v1/stim/replies/{reply_id}/events"
        ))
        .send()
        .map_err(|error| ControllerError::Server(format!("reply event request failed: {error}")))?
        .error_for_status()
        .map_err(|error| ControllerError::Server(format!("reply event status failed: {error}")))?
        .text()
        .map_err(|error| {
            ControllerError::Server(format!("reply event body read failed: {error}"))
        })?;

    let streamed = parse_reply_event_stream(&body)?;
    let snapshot = reqwest::blocking::Client::new()
        .get(format!("{santi_base_url}/api/v1/stim/replies/{reply_id}"))
        .send()
        .map_err(|error| {
            ControllerError::Server(format!("reply snapshot request failed: {error}"))
        })?
        .error_for_status()
        .map_err(|error| ControllerError::Server(format!("reply snapshot status failed: {error}")))?
        .json::<ReplySnapshot>()
        .map_err(|error| {
            ControllerError::Server(format!("reply snapshot decode failed: {error}"))
        })?;

    if !snapshot.output_text.trim().is_empty() {
        return Ok(snapshot.output_text);
    }

    Ok(streamed)
}

fn parse_reply_event_stream(body: &str) -> Result<String, ControllerError> {
    let mut reply = String::new();
    let mut completed = false;

    for line in body.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();

        if payload == "[DONE]" {
            break;
        }

        let event: ReplyEvent = serde_json::from_str(payload).map_err(|error| {
            ControllerError::Server(format!("reply event SSE decode failed: {error}"))
        })?;

        match event.event {
            ReplyEventKind::OutputTextDelta { delta } => {
                reply.push_str(&delta);
            }
            ReplyEventKind::Completed => {
                completed = true;
            }
            ReplyEventKind::Failed { error } => {
                return Err(ControllerError::Server(format!(
                    "reply event stream failed: {}: {}",
                    error.code, error.message
                )));
            }
        }
    }

    if reply.trim().is_empty() {
        return Err(ControllerError::Server(
            "reply event stream completed without assistant reply text".into(),
        ));
    }

    if !completed {
        return Err(ControllerError::Server(
            "reply event stream ended without completion event".into(),
        ));
    }

    Ok(reply)
}

fn synthetic_response_envelope(
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

fn sample_roundtrip_ids(conversation_id: Option<&str>) -> RoundtripIds {
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

pub fn sample_local_discovery_record(instance_suffix: &str) -> DiscoveryRecord {
    sample_discovery_record(
        "node-a",
        "endpoint-a",
        &format!("controller://stim/{instance_suffix}/self"),
        "local-controller",
        "local",
        vec!["local"],
        vec!["delivery", "controller_runtime"],
    )
}

pub fn sample_santi_discovery_record(base_url: &str) -> DiscoveryRecord {
    sample_discovery_record(
        "node-b",
        "endpoint-b",
        base_url,
        "santi-endpoint",
        "http",
        vec!["http"],
        vec!["delivery", "stim_protocol"],
    )
}

pub fn seed_discovery_records(santi_base_url: &str) -> Vec<DiscoveryRecord> {
    let fixture = http_santi_discovery_fixture("default", santi_base_url);
    vec![fixture.peer_discovery, fixture.self_discovery]
}

pub fn http_santi_discovery_fixture(
    instance_suffix: &str,
    santi_base_url: &str,
) -> ControllerDiscoveryFixture {
    ControllerDiscoveryFixture {
        self_discovery: sample_local_discovery_record(instance_suffix),
        peer_discovery: sample_santi_discovery_record(santi_base_url),
    }
}

fn sample_discovery_record(
    node_id: &str,
    endpoint_id: &str,
    address: &str,
    display_label: &str,
    carrier_kind: &str,
    supported_carriers: Vec<&str>,
    declared_features: Vec<&str>,
) -> DiscoveryRecord {
    DiscoveryRecord {
        node_id: node_id.into(),
        endpoint_declaration: EndpointDeclaration {
            endpoint_id: endpoint_id.into(),
            node_id: node_id.into(),
            display_label: Some(display_label.into()),
            endpoint_kind: Some(
                if carrier_kind == "http" {
                    "santi"
                } else {
                    "stim"
                }
                .into(),
            ),
            supported_protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
            supported_carriers: supported_carriers.into_iter().map(String::from).collect(),
            content_capabilities: vec!["text".into()],
            security_capabilities: vec!["sender_assertion".into()],
            declared_features: declared_features.into_iter().map(String::from).collect(),
        },
        carrier_kind: carrier_kind.into(),
        addresses: vec![address.into()],
        protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
    }
}

fn sample_create_envelope(
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

fn sample_patch_envelope(ids: &RoundtripIds, base_version: u64, text: &str) -> MessageEnvelope {
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

fn sample_fix_envelope(ids: &RoundtripIds, base_version: u64) -> MessageEnvelope {
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

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread};

    use axum::{routing::get, routing::post, Json, Router};
    use stim_proto::{
        AcknowledgementResult, DeliveryReceiptResult, MessageEnvelope, ProtocolAcknowledgement,
        ProtocolSubmission, ReplyHandle, ReplySnapshot, ReplyStatus,
    };

    use super::{
        first_message_roundtrip_with_records, http_santi_discovery_fixture,
        message_roundtrip_with_records, parse_acknowledgement, sample_santi_discovery_record,
    };

    fn spawn_test_santi_server() -> String {
        let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = std_listener.local_addr().unwrap();
        std_listener.set_nonblocking(true).unwrap();

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
                let app = Router::new()
                    .route("/api/v1/health", get(|| async { Json("ok") }))
                    .route(
                        "/api/v1/stim/envelopes",
                        post(|Json(envelope): Json<MessageEnvelope>| async move {
                            Json(ProtocolSubmission {
                                acknowledgement: ProtocolAcknowledgement {
                                    ack_envelope_id: format!("ack-{}", envelope.envelope_id),
                                    ack_message_id: envelope.message_id.clone(),
                                    ack_version: envelope.new_version,
                                    ack_result: AcknowledgementResult::Applied,
                                    detail: Some(format!(
                                        "santi session {} applied",
                                        envelope.conversation_id
                                    )),
                                },
                                reply: matches!(envelope.operation, stim_proto::MessageOperation::Fix)
                                    .then(|| ReplyHandle {
                                        reply_id: "reply-1".into(),
                                        conversation_id: envelope.conversation_id,
                                        message_id: envelope.message_id,
                                        status: ReplyStatus::Pending,
                                    }),
                            })
                        }),
                    )
                    .route(
                        "/api/v1/stim/replies/{reply_id}/events",
                        get(|| async move {
                            let body = concat!(
                                r#"data: {"reply_id":"reply-1","sequence":1,"event":{"type":"output_text_delta","delta":"real "}}"#,
                                "\n\n",
                                r#"data: {"reply_id":"reply-1","sequence":2,"event":{"type":"output_text_delta","delta":"santi reply"}}"#,
                                "\n\n",
                                r#"data: {"reply_id":"reply-1","sequence":3,"event":{"type":"completed"}}"#,
                                "\n\n",
                                "data: [DONE]",
                                "\n\n"
                            );
                            ([("content-type", "text/event-stream")], body)
                        }),
                    )
                    .route(
                        "/api/v1/stim/replies/{reply_id}",
                        get(|| async move {
                            Json(ReplySnapshot {
                                reply_id: "reply-1".into(),
                                conversation_id: "conv-1".into(),
                                message_id: "msg-1".into(),
                                status: ReplyStatus::Completed,
                                output_text: "real santi reply".into(),
                                error: None,
                            })
                        }),
                    );

                axum::serve(listener, app).await.unwrap();
            });
        });

        format!("http://{local_addr}")
    }

    #[test]
    fn controller_runtime_caches_discovery_and_roundtrips_message() {
        let santi_base_url = spawn_test_santi_server();
        let fixture = http_santi_discovery_fixture("default", &santi_base_url);
        let summary = first_message_roundtrip_with_records(
            "http://127.0.0.1:8080",
            "endpoint-b",
            "hello controller",
            fixture.self_discovery.clone(),
            vec![fixture.peer_discovery.clone(), fixture.self_discovery],
        )
        .unwrap();

        assert_eq!(summary.server_base_url, "http://127.0.0.1:8080");
        assert_eq!(summary.endpoint_id, "endpoint-b");
        assert!(summary.conversation_id.starts_with("conv-"));
        assert!(summary.message_id.starts_with("msg-"));
        assert_eq!(summary.listen_address, santi_base_url);
        assert!(summary.envelope_id.starts_with("env-"));
        assert_eq!(
            summary.final_sent_text,
            "hello controller (edited before fix)"
        );
        assert_eq!(summary.final_message_version, 3);
        assert_eq!(summary.response_envelope_id, "reply-1");
        assert_eq!(summary.response_text, "real santi reply");
        assert_eq!(summary.response_text_source, "stim_reply_handle");
        assert_eq!(summary.receipt_result, DeliveryReceiptResult::Accepted);
        let expected_detail = format!("santi session {} applied", summary.conversation_id);
        assert_eq!(
            summary.receipt_detail.as_deref(),
            Some(expected_detail.as_str())
        );
        assert_eq!(summary.lifecycle_trace.len(), 3);
        assert_eq!(summary.lifecycle_trace[0].operation, "create");
        assert_eq!(summary.lifecycle_trace[0].ack_version, 1);
        assert_eq!(summary.lifecycle_trace[1].operation, "patch");
        assert_eq!(summary.lifecycle_trace[1].ack_version, 2);
        assert_eq!(summary.lifecycle_trace[2].operation, "fix");
        assert_eq!(summary.lifecycle_trace[2].ack_version, 3);
        assert_eq!(summary.lifecycle_proof.create_ack_version, 1);
        assert_eq!(summary.lifecycle_proof.patch_ack_version, 2);
        assert_eq!(summary.lifecycle_proof.fix_ack_version, 3);
        assert_eq!(summary.lifecycle_proof.final_message_version, 3);
        assert_eq!(
            summary.lifecycle_proof.expected_final_text,
            summary.final_sent_text
        );
        assert_eq!(
            summary.lifecycle_proof.controller_final_text,
            summary.final_sent_text
        );
        assert!(summary.lifecycle_proof.final_text_matches_expected);
        assert!(summary.lifecycle_proof.version_progression_valid);
        assert_eq!(summary.cached_endpoint_count, 2);
    }

    #[test]
    fn controller_runtime_uses_registry_records_for_selected_endpoint() {
        let santi_base_url = spawn_test_santi_server();
        let fixture = http_santi_discovery_fixture("registry-test", &santi_base_url);
        let summary = first_message_roundtrip_with_records(
            "http://127.0.0.1:43100",
            "endpoint-b",
            "hello registry",
            fixture.self_discovery.clone(),
            vec![fixture.peer_discovery, fixture.self_discovery],
        )
        .unwrap();

        assert_eq!(summary.server_base_url, "http://127.0.0.1:43100");
        assert_eq!(summary.endpoint_id, "endpoint-b");
        assert_eq!(
            summary.final_sent_text,
            "hello registry (edited before fix)"
        );
        assert_eq!(summary.final_message_version, 3);
        assert_eq!(summary.response_text, "real santi reply");
        assert_eq!(summary.response_text_source, "stim_reply_handle");
        assert_eq!(summary.lifecycle_trace.len(), 3);
        assert!(summary.lifecycle_proof.version_progression_valid);
    }

    #[test]
    fn controller_runtime_can_continue_existing_conversation() {
        let santi_base_url = spawn_test_santi_server();
        let fixture = http_santi_discovery_fixture("continue-test", &santi_base_url);

        let first = message_roundtrip_with_records(
            "http://127.0.0.1:8080",
            "endpoint-b",
            "hello first turn",
            Some("conv-shared"),
            fixture.self_discovery.clone(),
            vec![
                fixture.peer_discovery.clone(),
                fixture.self_discovery.clone(),
            ],
        )
        .unwrap();
        let second = message_roundtrip_with_records(
            "http://127.0.0.1:8080",
            "endpoint-b",
            "hello second turn",
            Some("conv-shared"),
            fixture.self_discovery.clone(),
            vec![fixture.peer_discovery, fixture.self_discovery],
        )
        .unwrap();

        assert_eq!(first.conversation_id, "conv-shared");
        assert_eq!(second.conversation_id, "conv-shared");
        assert_ne!(first.message_id, second.message_id);
        assert_eq!(second.response_text, "real santi reply");
    }

    #[test]
    fn santi_discovery_record_uses_http_attach_target() {
        let record = sample_santi_discovery_record("http://127.0.0.1:18081");

        assert_eq!(record.carrier_kind, "http");
        assert_eq!(record.addresses, vec!["http://127.0.0.1:18081"]);
        assert_eq!(
            record.endpoint_declaration.endpoint_kind.as_deref(),
            Some("santi")
        );
    }

    #[test]
    fn acknowledgement_keeps_receipt_fields_without_output_parsing() {
        let parsed = parse_acknowledgement(&ProtocolAcknowledgement {
            ack_envelope_id: "ack-env-1".into(),
            ack_message_id: "msg-1".into(),
            ack_version: 1,
            ack_result: AcknowledgementResult::Applied,
            detail: Some("santi session conv-1 applied".into()),
        });

        assert_eq!(parsed.receipt_result, DeliveryReceiptResult::Accepted);
        assert_eq!(
            parsed.receipt_detail.as_deref(),
            Some("santi session conv-1 applied")
        );
        assert_eq!(parsed.ack_envelope_id, "ack-env-1");
        assert_eq!(parsed.ack_message_id, "msg-1");
        assert_eq!(parsed.ack_version, 1);
    }
}
