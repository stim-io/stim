use stim_agents::{
    schema::{AgentInstanceSnapshot, AgentInstanceState},
    stim_server::{capabilities, discovery_record},
};

#[test]
fn describes_santi_facts() {
    assert_eq!(capabilities(&santi_snapshot()), vec!["santi"]);
}

#[test]
fn uses_delivery_endpoint_id() {
    let record = discovery_record(&santi_snapshot());

    assert_eq!(record.node_id, "local-santi");
    assert_eq!(record.endpoint_declaration.endpoint_id, "endpoint-b");
    assert_eq!(record.addresses, vec!["http://127.0.0.1:18081".to_string()]);
    assert_eq!(record.carrier_kind, "http");
}

fn santi_snapshot() -> AgentInstanceSnapshot {
    AgentInstanceSnapshot {
        id: "local-santi".into(),
        agent_id: "santi".into(),
        participant_id: "participant-santi".into(),
        delivery_endpoint_id: "endpoint-b".into(),
        label: "Local Santi".into(),
        agent_kind: "santi".into(),
        managed: false,
        active: true,
        state: AgentInstanceState::Ready,
        endpoint: Some("http://127.0.0.1:18081".into()),
        profile: Some("local".into()),
        process: None,
        service: None,
        config: None,
        provider: None,
        provider_probe: None,
        runtime: None,
        last_probe_at: "1-000".into(),
        detail: None,
    }
}
