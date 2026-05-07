use serde::{Deserialize, Serialize};

pub const DEFAULT_SIDECAR_NAMESPACE: &str = "default";
pub const SIDECAR_NAMESPACE_ENV: &str = "STIM_SIDECAR_NAMESPACE";
pub const LEGACY_IPC_NAMESPACE_ENV: &str = "STIM_IPC_NAMESPACE";

pub const DEFAULT_IPC_NAMESPACE: &str = DEFAULT_SIDECAR_NAMESPACE;
pub const IPC_NAMESPACE_ENV: &str = LEGACY_IPC_NAMESPACE_ENV;

const CONTROL_PREFIX: &str = "stim://control";
const AGENTS_RUNTIME_SNAPSHOT_TOPIC: &str = "agents/runtime/snapshot";
const AGENTS_RUNTIME_HEARTBEAT_TOPIC: &str = "agents/runtime/heartbeat";
const CONTROLLER_RUNTIME_SNAPSHOT_TOPIC: &str = "controller/runtime/snapshot";
const CONTROLLER_RUNTIME_HEARTBEAT_TOPIC: &str = "controller/runtime/heartbeat";

pub fn namespace_or_default(namespace: Option<&str>) -> &str {
    namespace
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_SIDECAR_NAMESPACE)
}

pub fn namespaced_control_topic(namespace: &str, topic: &str) -> String {
    format!(
        "{CONTROL_PREFIX}/{}/{}",
        namespace_or_default(Some(namespace)),
        topic
    )
}

pub fn controller_runtime_snapshot_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, CONTROLLER_RUNTIME_SNAPSHOT_TOPIC)
}

pub fn controller_runtime_heartbeat_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, CONTROLLER_RUNTIME_HEARTBEAT_TOPIC)
}

pub fn agents_runtime_snapshot_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, AGENTS_RUNTIME_SNAPSHOT_TOPIC)
}

pub fn agents_runtime_heartbeat_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, AGENTS_RUNTIME_HEARTBEAT_TOPIC)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentsRuntimeState {
    Starting,
    Ready,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsRuntimeSnapshot {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub state: AgentsRuntimeState,
    pub http_base_url: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsRuntimeHeartbeat {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub sequence: u64,
    pub state: AgentsRuntimeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerRuntimeState {
    Starting,
    Ready,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerRuntimeSnapshot {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub state: ControllerRuntimeState,
    pub http_base_url: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerRuntimeHeartbeat {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub sequence: u64,
    pub state: ControllerRuntimeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RendererDeliveryLaunchBridge {
    pub namespace: String,
    pub renderer_url: String,
    pub source: String,
    pub published_at: String,
}
