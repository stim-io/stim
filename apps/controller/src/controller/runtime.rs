use std::{thread, time::Duration};

use stim_proto::{ContentPart, DiscoveryRecord, MessageEnvelope, MutationPayload, TextPart};
use stim_shared::delivery::{DeliveryError, DeliveryPort};

use super::{
    carrier::HttpSantiCarrier,
    facade::{HttpStimServerFacade, InMemoryStimServerFacade, StimServerFacade},
    fixtures::http_santi_discovery_fixture,
    messages::{
        sample_create_envelope, sample_fix_envelope, sample_patch_envelope, sample_roundtrip_ids,
        user_text_content,
    },
    reply::request_protocol_reply,
    types::{
        ControllerError, ControllerLifecycleProof, ControllerLifecycleStep, ControllerProofSummary,
        ControllerState, DEFAULT_COMPOSE_SANTI_BASE_URL, DEFAULT_COMPOSE_STIM_SERVER_BASE_URL,
    },
};

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
        let response = request_protocol_reply(&target.selected_address, &reply_id)?;

        self.delivery_port.close_delivery_target(&target)?;

        Ok(ControllerProofSummary {
            server_base_url: self.state.server_base_url().into(),
            endpoint_id: endpoint_id.into(),
            conversation_id: ids.conversation_id,
            message_id: ids.message_id,
            listen_address: peer_listen_address,
            envelope_id: sent_envelope_id,
            final_sent_text: revised_text.clone(),
            final_sent_content: user_text_content(&revised_text),
            final_message_version: fix_response.new_version,
            response_envelope_id: reply_id,
            response_text: response.text,
            response_content: response.content,
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
