use serde::Deserialize;

use crate::schema::{
    AgentInstanceSnapshot, AgentInstanceState, AgentProcessFacts, AgentProfileSecretState,
    AgentProfileSummary, SantiCapabilityFacts, SantiConfigFacts, SantiProviderFacts,
    SantiProviderProbeFacts, SantiRuntimeFacts, SantiServiceFacts,
};

use super::{
    config::{non_empty_env, SantiLaunchConfig, SantiProfileSecretConfig},
    AgentRegistryError,
};

pub(super) struct AgentInstanceConfig {
    pub(super) id: String,
    pub(super) agent_id: String,
    pub(super) participant_id: String,
    pub(super) delivery_endpoint_id: String,
    pub(super) label: String,
    pub(super) namespace: String,
    pub(super) endpoint: String,
    pub(super) profile: Option<String>,
    pub(super) managed: bool,
    pub(super) launch: Option<SantiLaunchConfig>,
}

#[derive(Clone, Debug)]
pub(super) struct AgentProfileConfig {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) launch_profile: String,
    pub(super) provider: AgentProfileProviderConfig,
}

#[derive(Clone, Debug)]
pub(super) struct AgentProfileProviderConfig {
    pub(super) api: String,
    pub(super) model: String,
    pub(super) gateway_base_url: String,
    pub(super) api_key: SantiProfileSecretConfig,
}

pub(super) struct SnapshotInput {
    pub(super) state: AgentInstanceState,
    pub(super) active: bool,
    pub(super) process: Option<AgentProcessFacts>,
    pub(super) last_probe_at: String,
    pub(super) service: Option<SantiServiceFacts>,
    pub(super) config: Option<SantiConfigFacts>,
    pub(super) provider: Option<SantiProviderFacts>,
    pub(super) provider_probe: Option<SantiProviderProbeFacts>,
    pub(super) runtime: Option<SantiRuntimeFacts>,
    pub(super) detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SantiMetaResponse {
    api_version: Option<String>,
    service_name: Option<String>,
    service_version: Option<String>,
    mode: Option<String>,
    launch_profile: Option<String>,
    bind_addr: Option<String>,
    capabilities: Option<SantiCapabilityFacts>,
    pub(super) provider: Option<SantiProviderFacts>,
    pub(super) runtime: Option<SantiRuntimeFacts>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SantiConfigApplyResponse {
    pub(super) event_id: String,
    pub(super) config_version: u64,
}

impl AgentInstanceConfig {
    pub(super) fn provider_probe_url(&self) -> String {
        format!(
            "{}/api/v1/admin/provider/probe",
            self.endpoint.trim_end_matches('/')
        )
    }

    pub(super) fn config_url(&self) -> String {
        format!(
            "{}/api/v1/admin/config",
            self.endpoint.trim_end_matches('/')
        )
    }

    pub(super) fn snapshot(&self, input: SnapshotInput) -> AgentInstanceSnapshot {
        AgentInstanceSnapshot {
            id: self.id.clone(),
            agent_id: self.agent_id.clone(),
            participant_id: self.participant_id.clone(),
            delivery_endpoint_id: self.delivery_endpoint_id.clone(),
            label: self.label.clone(),
            agent_kind: "santi".into(),
            managed: self.managed,
            active: input.active,
            state: input.state,
            endpoint: Some(self.endpoint.clone()),
            profile: self
                .profile
                .clone()
                .or_else(|| Some(self.namespace.clone())),
            process: input.process,
            service: input.service,
            config: input.config,
            provider: input.provider,
            provider_probe: input.provider_probe,
            runtime: input.runtime,
            last_probe_at: input.last_probe_at,
            detail: input.detail,
        }
    }
}

impl AgentProfileConfig {
    pub(super) fn summary(&self) -> AgentProfileSummary {
        AgentProfileSummary {
            id: self.id.clone(),
            label: self.label.clone(),
            launch_profile: self.launch_profile.clone(),
            provider: SantiProviderFacts {
                api: self.provider.api.clone(),
                model: self.provider.model.clone(),
                gateway_base_url: Some(self.provider.gateway_base_url.clone()),
            },
            secret_state: match self.secret_value() {
                Ok(_) => AgentProfileSecretState::Available,
                Err(_) => AgentProfileSecretState::Missing,
            },
        }
    }

    pub(super) fn secret_value(&self) -> Result<String, AgentRegistryError> {
        match &self.provider.api_key {
            SantiProfileSecretConfig::Value(value) => {
                let value = value.trim();
                if value.is_empty() {
                    Err(AgentRegistryError::SecretMissing(self.id.clone()))
                } else {
                    Ok(value.to_string())
                }
            }
            SantiProfileSecretConfig::Env(key) => {
                non_empty_env(key).ok_or_else(|| AgentRegistryError::SecretMissing(self.id.clone()))
            }
        }
    }
}

impl SantiMetaResponse {
    pub(super) fn service_facts(&self) -> SantiServiceFacts {
        SantiServiceFacts {
            api_version: self.api_version.clone(),
            service_name: self.service_name.clone().unwrap_or_else(|| "santi".into()),
            service_version: self.service_version.clone(),
            mode: self.mode.clone(),
            launch_profile: self.launch_profile.clone(),
            bind_addr: self.bind_addr.clone(),
            capabilities: self.capabilities.clone(),
        }
    }
}
