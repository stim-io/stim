use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use stim_proto::{
    AcknowledgementResult, ContentPart, DeliveryReceipt, DeliveryReceiptResult, DiscoveryRecord,
    EndpointDeclaration, MessageContent, MessageEnvelope, MessageOperation, MessageState,
    MutationPayload, ProtocolAcknowledgement, TextPart,
};
use stim_shared::delivery::{DeliveryError, DeliveryPort};

const DEFAULT_COMPOSE_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_COMPOSE_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerDiscoveryFixture {
    pub self_discovery: DiscoveryRecord,
    pub peer_discovery: DiscoveryRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerProofSummary {
    pub server_base_url: String,
    pub endpoint_id: String,
    pub listen_address: String,
    pub envelope_id: String,
    pub response_envelope_id: String,
    pub response_text: String,
    pub receipt_result: DeliveryReceiptResult,
    pub cached_endpoint_count: usize,
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

        let acknowledgement = self
            .client
            .post(format!("{}/api/v1/stim/envelopes", target.selected_address))
            .json(&envelope)
            .send()
            .map_err(|error| DeliveryError::Remote(format!("santi request failed: {error}")))?
            .error_for_status()
            .map_err(|error| DeliveryError::Remote(format!("santi status failed: {error}")))?
            .json::<ProtocolAcknowledgement>()
            .map_err(|error| DeliveryError::Remote(format!("santi decode failed: {error}")))?;

        self.enqueue_response(synthetic_response_envelope(
            &envelope,
            target,
            &acknowledgement,
        ))?;

        Ok(DeliveryReceipt {
            envelope_id: envelope.envelope_id,
            result: acknowledgement_to_receipt_result(&acknowledgement),
            detail: acknowledgement.detail,
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
        envelope: MessageEnvelope,
    ) -> Result<ControllerProofSummary, ControllerError> {
        let peer_discovery = self.server_facade.discover_endpoint(endpoint_id)?;
        let self_discovery = self.self_discovery.clone();

        let peer_listen_address = first_listen_address(&peer_discovery)?;
        let self_listen_address = first_listen_address(&self_discovery)?;
        let sent_envelope_id = envelope.envelope_id.clone();

        self.state.cache_discovery_record(peer_discovery.clone());
        self.state.cache_discovery_record(self_discovery.clone());

        let target = self.delivery_port.open_delivery_target(&peer_discovery)?;
        let receipt = self.delivery_port.send_envelope(&target, envelope)?;
        self.delivery_port.close_delivery_target(&target)?;
        let received_by_self = receive_with_retry(&self.delivery_port, &self_listen_address)?;
        let response_text = extract_text(&received_by_self)?;

        Ok(ControllerProofSummary {
            server_base_url: self.state.server_base_url().into(),
            endpoint_id: endpoint_id.into(),
            listen_address: peer_listen_address,
            envelope_id: sent_envelope_id,
            response_envelope_id: received_by_self.envelope_id,
            response_text,
            receipt_result: receipt.result,
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
    let facade = InMemoryStimServerFacade::new(server_base_url, records);
    let carrier = http_santi_carrier(&self_discovery)?;

    let mut runtime = ControllerRuntime::new(facade, carrier, self_discovery);
    runtime.deliver_to_endpoint(endpoint_id, sample_request_envelope(text))
}

pub fn first_message_roundtrip(text: &str) -> Result<ControllerProofSummary, ControllerError> {
    let fixture = http_santi_discovery_fixture("default", DEFAULT_COMPOSE_SANTI_BASE_URL);
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
    let facade = HttpStimServerFacade::new(server_base_url);
    let carrier = http_santi_carrier(&self_discovery)?;

    let mut runtime = ControllerRuntime::new(facade, carrier, self_discovery);
    runtime.deliver_to_endpoint(endpoint_id, sample_request_envelope(text))
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

fn acknowledgement_output_text(acknowledgement: &ProtocolAcknowledgement) -> String {
    if let Some(detail) = &acknowledgement.detail {
        if let Some((_, output)) = detail.rsplit_once("output=") {
            return output.to_string();
        }

        return detail.clone();
    }

    format!("santi acknowledgement: {:?}", acknowledgement.ack_result).to_lowercase()
}

fn synthetic_response_envelope(
    request: &MessageEnvelope,
    target: &stim_proto::DeliveryTarget,
    acknowledgement: &ProtocolAcknowledgement,
) -> MessageEnvelope {
    sample_text_envelope(
        &acknowledgement.ack_envelope_id,
        &acknowledgement.ack_message_id,
        &request.conversation_id,
        &target.node_id,
        "endpoint-b",
        &acknowledgement_output_text(acknowledgement),
        MessageState::Fixed,
        acknowledgement.ack_version,
    )
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

fn sample_request_envelope(text: &str) -> MessageEnvelope {
    sample_text_envelope(
        "env-1",
        "msg-1",
        "conv-1",
        "node-a",
        "endpoint-a",
        text,
        MessageState::Pending,
        1,
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
    new_version: u64,
) -> MessageEnvelope {
    MessageEnvelope {
        protocol_version: stim_proto::CURRENT_PROTOCOL_VERSION.into(),
        envelope_id: envelope_id.into(),
        message_id: message_id.into(),
        conversation_id: conversation_id.into(),
        sender_node_id: sender_node_id.into(),
        sender_endpoint_id: sender_endpoint_id.into(),
        created_at: "2026-04-14T00:00:00Z".into(),
        session_bootstrap: None,
        sender_assertion: None,
        encryption_scope: None,
        recipient_key_refs: vec![],
        signature_ref: None,
        integrity_ref: None,
        state,
        operation: MessageOperation::Create,
        base_version: None,
        new_version,
        payload: MutationPayload::Create {
            content: MessageContent {
                parts: vec![ContentPart::Text(TextPart {
                    part_id: "part-1".into(),
                    revision: 1,
                    metadata: None,
                    text: text.into(),
                })],
                layout_hint: None,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread};

    use axum::{routing::get, routing::post, Json, Router};
    use stim_proto::{
        AcknowledgementResult, DeliveryReceiptResult, MessageEnvelope, ProtocolAcknowledgement,
    };

    use super::{
        first_message_roundtrip_with_records, http_santi_discovery_fixture,
        sample_santi_discovery_record,
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
                            Json(ProtocolAcknowledgement {
                                ack_envelope_id: format!("ack-{}", envelope.envelope_id),
                                ack_message_id: envelope.message_id,
                                ack_version: envelope.new_version,
                                ack_result: AcknowledgementResult::Applied,
                                detail: Some(format!(
                                    "santi session {} applied; output=mock santi reply",
                                    envelope.conversation_id
                                )),
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
        assert_eq!(summary.listen_address, santi_base_url);
        assert_eq!(summary.envelope_id, "env-1");
        assert_eq!(summary.response_envelope_id, "ack-env-1");
        assert_eq!(summary.response_text, "mock santi reply");
        assert_eq!(summary.receipt_result, DeliveryReceiptResult::Accepted);
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
        assert_eq!(summary.response_text, "mock santi reply");
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
}
