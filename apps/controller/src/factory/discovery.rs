use stim_proto::{DiscoveryRecord, EndpointDeclaration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControllerDiscoveryFixture {
    pub self_discovery: DiscoveryRecord,
    pub peer_discovery: DiscoveryRecord,
}

fn sample_local_discovery_record(instance_suffix: &str) -> DiscoveryRecord {
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

pub(crate) fn sample_santi_discovery_record(base_url: &str) -> DiscoveryRecord {
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

pub(crate) fn http_santi_discovery_fixture(
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
