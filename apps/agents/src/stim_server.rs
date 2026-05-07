use std::{collections::HashSet, env, time::Duration};

use reqwest::StatusCode;
use serde::Serialize;
use stim_proto::{DiscoveryRecord, EndpointDeclaration};

use crate::{
    schema::{AgentInstanceSnapshot, AgentInstanceState},
    state::AppState,
};

const STIM_SERVER_BASE_URL_ENV: &str = "STIM_AGENTS_STIM_SERVER_BASE_URL";
const STIM_SERVER_FALLBACK_ENV: &str = "STIM_SERVER_BASE_URL";
const DEFAULT_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const HEARTBEAT_INTERVAL_ENV: &str = "STIM_AGENTS_HEARTBEAT_INTERVAL_MS";

pub fn spawn_registration_loop(state: AppState) {
    let base_url = stim_server_base_url();
    let heartbeat_interval = heartbeat_interval();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut registered_instances = HashSet::new();

        loop {
            sync_once(&client, &base_url, &state, &mut registered_instances).await;
            tokio::time::sleep(heartbeat_interval).await;
        }
    });
}

async fn sync_once(
    client: &reqwest::Client,
    base_url: &str,
    state: &AppState,
    registered_instances: &mut HashSet<String>,
) {
    let snapshots = state.registry().list_instances().await;
    for snapshot in snapshots {
        if publish_discovery_record(client, base_url, &snapshot)
            .await
            .is_err()
        {
            continue;
        }

        if registered_instances.contains(&snapshot.id) {
            match heartbeat_instance(client, base_url, &snapshot).await {
                Ok(()) => {}
                Err(status) if status == StatusCode::NOT_FOUND => {
                    if register_instance(client, base_url, &snapshot).await.is_ok() {
                        registered_instances.insert(snapshot.id);
                    }
                }
                Err(_) => {}
            }
        } else if register_instance(client, base_url, &snapshot).await.is_ok() {
            registered_instances.insert(snapshot.id);
        }
    }
}

async fn publish_discovery_record(
    client: &reqwest::Client,
    base_url: &str,
    snapshot: &AgentInstanceSnapshot,
) -> Result<(), StatusCode> {
    let record = discovery_record(snapshot);
    let endpoint_id = record.endpoint_declaration.endpoint_id.clone();
    let response = client
        .put(format!(
            "{}/api/v1/discovery/endpoints/{}",
            base_url.trim_end_matches('/'),
            endpoint_id
        ))
        .json(&record)
        .send()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(response.status())
    }
}

async fn register_instance(
    client: &reqwest::Client,
    base_url: &str,
    snapshot: &AgentInstanceSnapshot,
) -> Result<(), StatusCode> {
    let response = client
        .put(format!(
            "{}/api/v1/agents/instances/{}",
            base_url.trim_end_matches('/'),
            snapshot.id
        ))
        .json(&RegistrationPayload::from(snapshot))
        .send()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(response.status())
    }
}

async fn heartbeat_instance(
    client: &reqwest::Client,
    base_url: &str,
    snapshot: &AgentInstanceSnapshot,
) -> Result<(), StatusCode> {
    let response = client
        .post(format!(
            "{}/api/v1/agents/instances/{}/heartbeat",
            base_url.trim_end_matches('/'),
            snapshot.id
        ))
        .json(&HeartbeatPayload::from(snapshot))
        .send()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(response.status())
    }
}

#[derive(Serialize)]
struct RegistrationPayload {
    agent_id: String,
    instance_id: String,
    participant_id: String,
    delivery_endpoint_id: String,
    label: String,
    agent_kind: String,
    endpoint: Option<String>,
    profile: Option<String>,
    capabilities: Vec<String>,
    status: ServerAgentStatus,
    detail: Option<String>,
}

impl From<&AgentInstanceSnapshot> for RegistrationPayload {
    fn from(snapshot: &AgentInstanceSnapshot) -> Self {
        Self {
            agent_id: snapshot.agent_id.clone(),
            instance_id: snapshot.id.clone(),
            participant_id: snapshot.participant_id.clone(),
            delivery_endpoint_id: snapshot.delivery_endpoint_id.clone(),
            label: snapshot.label.clone(),
            agent_kind: snapshot.agent_kind.clone(),
            endpoint: snapshot.endpoint.clone(),
            profile: snapshot.profile.clone(),
            capabilities: capabilities(snapshot),
            status: ServerAgentStatus::from(&snapshot.state),
            detail: snapshot.detail.clone(),
        }
    }
}

#[derive(Serialize)]
struct HeartbeatPayload {
    agent_id: String,
    instance_id: String,
    participant_id: String,
    delivery_endpoint_id: String,
    endpoint: Option<String>,
    status: ServerAgentStatus,
    detail: Option<String>,
}

impl From<&AgentInstanceSnapshot> for HeartbeatPayload {
    fn from(snapshot: &AgentInstanceSnapshot) -> Self {
        Self {
            agent_id: snapshot.agent_id.clone(),
            instance_id: snapshot.id.clone(),
            participant_id: snapshot.participant_id.clone(),
            delivery_endpoint_id: snapshot.delivery_endpoint_id.clone(),
            endpoint: snapshot.endpoint.clone(),
            status: ServerAgentStatus::from(&snapshot.state),
            detail: snapshot.detail.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum ServerAgentStatus {
    Ready,
    Degraded,
    Unreachable,
}

impl From<&AgentInstanceState> for ServerAgentStatus {
    fn from(value: &AgentInstanceState) -> Self {
        match value {
            AgentInstanceState::Ready => Self::Ready,
            AgentInstanceState::Degraded => Self::Degraded,
            AgentInstanceState::Unreachable => Self::Unreachable,
        }
    }
}

pub fn capabilities(snapshot: &AgentInstanceSnapshot) -> Vec<String> {
    let mut capabilities = vec!["santi".to_string()];
    if snapshot.service.is_some() {
        capabilities.push("service-facts".to_string());
    }
    if snapshot.provider.is_some() {
        capabilities.push("provider-facts".to_string());
    }
    if snapshot.provider_probe.is_some() {
        capabilities.push("provider-probe".to_string());
    }
    if snapshot.runtime.is_some() {
        capabilities.push("runtime-facts".to_string());
    }
    capabilities
}

pub fn discovery_record(snapshot: &AgentInstanceSnapshot) -> DiscoveryRecord {
    DiscoveryRecord {
        node_id: snapshot.id.clone(),
        endpoint_declaration: EndpointDeclaration {
            endpoint_id: snapshot.delivery_endpoint_id.clone(),
            node_id: snapshot.id.clone(),
            display_label: Some(snapshot.label.clone()),
            endpoint_kind: Some(snapshot.agent_kind.clone()),
            supported_protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
            supported_carriers: vec!["http".into()],
            content_capabilities: vec!["text".into()],
            security_capabilities: vec!["sender_assertion".into()],
            declared_features: vec![
                "delivery".into(),
                "stim_protocol".into(),
                "agent_instance_projection".into(),
            ],
        },
        carrier_kind: "http".into(),
        addresses: snapshot.endpoint.iter().cloned().collect(),
        protocol_versions: vec![stim_proto::CURRENT_PROTOCOL_VERSION.into()],
    }
}

fn stim_server_base_url() -> String {
    non_empty_env(STIM_SERVER_BASE_URL_ENV)
        .or_else(|| non_empty_env(STIM_SERVER_FALLBACK_ENV))
        .unwrap_or_else(|| DEFAULT_SERVER_BASE_URL.to_string())
}

fn heartbeat_interval() -> Duration {
    non_empty_env(HEARTBEAT_INTERVAL_ENV)
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(5))
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
