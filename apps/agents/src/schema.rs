use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentInstanceListResponse {
    pub active_instance_id: String,
    pub instances: Vec<AgentInstanceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProfileListResponse {
    pub profiles: Vec<AgentProfileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProfileSummary {
    pub id: String,
    pub label: String,
    pub launch_profile: String,
    pub provider: SantiProviderFacts,
    pub secret_state: AgentProfileSecretState,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProfileSecretState {
    Available,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSelectionRequest {
    pub instance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProfileApplyRequest {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProfileApplyResponse {
    pub event_id: String,
    pub instance_id: String,
    pub profile_id: String,
    pub status: AgentProfileApplyStatus,
    pub santi_event_id: String,
    pub config_version: u64,
    pub snapshot: AgentInstanceSnapshot,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProfileApplyStatus {
    Applied,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSelectionResponse {
    pub active_instance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProviderProbeResponse {
    pub instance_id: String,
    pub provider_probe: SantiProviderProbeFacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentInstanceActionResponse {
    pub event_id: String,
    pub instance_id: String,
    pub action: AgentInstanceAction,
    pub status: AgentInstanceActionStatus,
    pub snapshot: AgentInstanceSnapshot,
    pub process_result: Option<AgentProcessStopFacts>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentInstanceAction {
    Launch,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentInstanceActionStatus {
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProcessFacts {
    pub pid: u32,
    pub launched_by_agents: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentProcessStopFacts {
    pub already_stopped: bool,
    pub matched_pids: Vec<u32>,
    pub stopped_pids: Vec<u32>,
    pub forced_pids: Vec<u32>,
    pub remaining_pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentInstanceSnapshot {
    pub id: String,
    pub agent_id: String,
    pub participant_id: String,
    pub delivery_endpoint_id: String,
    pub label: String,
    pub agent_kind: String,
    pub managed: bool,
    pub active: bool,
    pub state: AgentInstanceState,
    pub endpoint: Option<String>,
    pub profile: Option<String>,
    pub process: Option<AgentProcessFacts>,
    pub service: Option<SantiServiceFacts>,
    pub config: Option<SantiConfigFacts>,
    pub provider: Option<SantiProviderFacts>,
    pub provider_probe: Option<SantiProviderProbeFacts>,
    pub runtime: Option<SantiRuntimeFacts>,
    pub last_probe_at: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AgentInstanceState {
    Ready,
    Degraded,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiServiceFacts {
    pub api_version: Option<String>,
    pub service_name: String,
    pub service_version: Option<String>,
    pub mode: Option<String>,
    pub launch_profile: Option<String>,
    pub bind_addr: Option<String>,
    pub capabilities: Option<SantiCapabilityFacts>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiCapabilityFacts {
    pub health: bool,
    pub sessions: bool,
    pub soul: bool,
    pub admin_hooks: bool,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiConfigFacts {
    pub config_version: u64,
    pub last_event_id: String,
    pub source: String,
    pub launch_profile: Option<String>,
    pub provider: SantiProviderFacts,
    pub runtime: SantiRuntimeFacts,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiProviderFacts {
    pub api: String,
    pub model: String,
    pub gateway_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiProviderProbeFacts {
    pub state: SantiProviderProbeState,
    pub checked_url: String,
    pub http_status: Option<u16>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SantiProviderProbeState {
    Ready,
    Degraded,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SantiRuntimeFacts {
    pub execution_root: String,
    pub runtime_root: String,
    pub standalone_sqlite_path: Option<String>,
}
