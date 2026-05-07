use std::{collections::HashMap, process::Child, sync::Arc, time::Duration};

use reqwest::StatusCode;
use tokio::sync::RwLock;

use crate::schema::{
    AgentInstanceAction, AgentInstanceActionResponse, AgentInstanceActionStatus,
    AgentInstanceSnapshot, AgentProcessStopFacts, AgentProfileApplyResponse,
    AgentProfileApplyStatus, AgentProfileSummary, SantiProviderProbeFacts,
};

mod config;
mod model;
mod probe;
mod process;

pub use config::{
    SantiInstanceConfig, SantiLaunchConfig, SantiProfileConfig, SantiProfileProviderConfig,
    SantiProfileSecretConfig,
};

use config::{
    configured_santi_instances, configured_santi_profiles, default_santi_profiles,
    validate_instances, validate_profiles,
};
use model::{AgentInstanceConfig, AgentProfileConfig, SantiConfigApplyResponse};
use process::{spawn_santi_instance, stop_launched_process};

#[derive(Clone)]
pub struct AppState {
    registry: Arc<AgentRegistry>,
}

impl AppState {
    pub fn from_env(namespace: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            registry: Arc::new(AgentRegistry::from_env(namespace)?),
        })
    }

    pub fn single_santi(namespace: &str, endpoint: String) -> Self {
        Self::santi_instances(
            namespace,
            vec![SantiInstanceConfig {
                agent_id: None,
                participant_id: None,
                delivery_endpoint_id: None,
                id: "local-santi".into(),
                label: "Local Santi".into(),
                endpoint,
                profile: None,
                managed: false,
                launch: None,
            }],
        )
        .expect("single test Santi instance config should be valid")
    }

    pub fn santi_instances(
        namespace: &str,
        instances: Vec<SantiInstanceConfig>,
    ) -> Result<Self, String> {
        Self::santi_instances_with_profiles(namespace, instances, default_santi_profiles())
    }

    pub fn santi_instances_with_profiles(
        namespace: &str,
        instances: Vec<SantiInstanceConfig>,
        profiles: Vec<SantiProfileConfig>,
    ) -> Result<Self, String> {
        Ok(Self {
            registry: Arc::new(AgentRegistry::new(namespace, instances, profiles)?),
        })
    }

    pub fn registry(&self) -> Arc<AgentRegistry> {
        self.registry.clone()
    }
}

pub struct AgentRegistry {
    client: reqwest::Client,
    instances: Vec<AgentInstanceConfig>,
    profiles: Vec<AgentProfileConfig>,
    active_instance_id: RwLock<String>,
    launched_processes: RwLock<HashMap<String, Child>>,
}

impl AgentRegistry {
    fn from_env(namespace: Option<&str>) -> Result<Self, String> {
        let namespace = namespace.unwrap_or("default");
        Self::new(
            namespace,
            configured_santi_instances(namespace)?,
            configured_santi_profiles()?,
        )
    }

    fn new(
        namespace: &str,
        instances: Vec<SantiInstanceConfig>,
        profiles: Vec<SantiProfileConfig>,
    ) -> Result<Self, String> {
        let instances = validate_instances(namespace, instances)?;
        let profiles = validate_profiles(profiles)?;
        let active_instance_id = instances
            .first()
            .map(|instance| instance.id.clone())
            .ok_or_else(|| "at least one Santi instance must be configured".to_string())?;

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .expect("failed to build agents HTTP client"),
            instances,
            profiles,
            active_instance_id: RwLock::new(active_instance_id),
            launched_processes: RwLock::new(HashMap::new()),
        })
    }

    pub async fn list_instances(&self) -> Vec<AgentInstanceSnapshot> {
        let active_instance_id = self.active_instance_id().await;
        let launched_pids = self.launched_pid_snapshot().await;
        let mut snapshots = Vec::with_capacity(self.instances.len());
        for instance in &self.instances {
            snapshots.push(
                self.probe_configured_instance(instance, &active_instance_id, &launched_pids)
                    .await,
            );
        }
        snapshots
    }

    pub fn list_profiles(&self) -> Vec<AgentProfileSummary> {
        self.profiles
            .iter()
            .map(AgentProfileConfig::summary)
            .collect()
    }

    pub async fn active_instance_id(&self) -> String {
        self.active_instance_id.read().await.clone()
    }

    pub async fn select_instance(&self, instance_id: &str) -> Result<String, AgentRegistryError> {
        let instance = self
            .find_instance(instance_id)
            .ok_or(AgentRegistryError::NotFound)?;
        let mut active_instance_id = self.active_instance_id.write().await;
        *active_instance_id = instance.id.clone();
        Ok(active_instance_id.clone())
    }

    pub async fn get_instance(&self, instance_id: &str) -> Option<AgentInstanceSnapshot> {
        let instance = self.find_instance(instance_id)?;
        let active_instance_id = self.active_instance_id().await;
        let launched_pids = self.launched_pid_snapshot().await;
        Some(
            self.probe_configured_instance(instance, &active_instance_id, &launched_pids)
                .await,
        )
    }

    pub async fn launch_instance(
        &self,
        instance_id: &str,
    ) -> Result<AgentInstanceActionResponse, AgentRegistryError> {
        let instance = self
            .find_instance(instance_id)
            .ok_or(AgentRegistryError::NotFound)?;

        if !instance.managed {
            return Err(AgentRegistryError::Unmanaged);
        }

        let launch = instance
            .launch
            .as_ref()
            .ok_or(AgentRegistryError::LaunchUnavailable)?;
        if self
            .launched_processes
            .read()
            .await
            .contains_key(&instance.id)
        {
            return Err(AgentRegistryError::AlreadyRunning);
        }
        let child = spawn_santi_instance(instance, launch)?;
        let pid = child.id();
        {
            let mut launched_processes = self.launched_processes.write().await;
            launched_processes.insert(instance.id.clone(), child);
        }

        let active_instance_id = self.active_instance_id().await;
        let launched_pids = self.launched_pid_snapshot().await;
        let snapshot = self
            .probe_configured_instance(instance, &active_instance_id, &launched_pids)
            .await;

        Ok(AgentInstanceActionResponse {
            event_id: action_event_id("launch"),
            instance_id: instance.id.clone(),
            action: AgentInstanceAction::Launch,
            status: AgentInstanceActionStatus::Completed,
            snapshot,
            process_result: None,
            detail: Some(format!("launched managed Santi instance with pid {pid}")),
        })
    }

    pub async fn stop_instance(
        &self,
        instance_id: &str,
    ) -> Result<AgentInstanceActionResponse, AgentRegistryError> {
        let instance = self
            .find_instance(instance_id)
            .ok_or(AgentRegistryError::NotFound)?;

        if !instance.managed {
            return Err(AgentRegistryError::Unmanaged);
        }

        let mut child = self
            .launched_processes
            .write()
            .await
            .remove(&instance.id)
            .ok_or(AgentRegistryError::NotRunning)?;
        let pid = child.id();
        let process_result = match stop_launched_process(pid) {
            Ok(result) => result,
            Err(error) => {
                let mut launched_processes = self.launched_processes.write().await;
                launched_processes.insert(instance.id.clone(), child);
                return Err(error);
            }
        };
        if process_result.remaining_pids.is_empty() {
            let _ = child.wait();
        } else {
            let mut launched_processes = self.launched_processes.write().await;
            launched_processes.insert(instance.id.clone(), child);
        }
        let active_instance_id = self.active_instance_id().await;
        let launched_pids = self.launched_pid_snapshot().await;
        let snapshot = self
            .probe_configured_instance(instance, &active_instance_id, &launched_pids)
            .await;

        Ok(AgentInstanceActionResponse {
            event_id: action_event_id("stop"),
            instance_id: instance.id.clone(),
            action: AgentInstanceAction::Stop,
            status: AgentInstanceActionStatus::Completed,
            snapshot,
            detail: Some(format!(
                "stopped managed Santi instance pid {pid}; remaining {:?}",
                process_result.remaining_pids
            )),
            process_result: Some(AgentProcessStopFacts::from(process_result)),
        })
    }

    pub async fn apply_profile(
        &self,
        instance_id: &str,
        profile_id: &str,
    ) -> Result<AgentProfileApplyResponse, AgentRegistryError> {
        let instance = self
            .find_instance(instance_id)
            .ok_or(AgentRegistryError::NotFound)?;
        let profile = self
            .find_profile(profile_id)
            .ok_or(AgentRegistryError::ProfileNotFound)?;
        let api_key = profile.secret_value()?;
        let apply_url = format!(
            "{}/api/v1/admin/config/apply",
            instance.endpoint.trim_end_matches('/')
        );
        let response = self
            .client
            .post(apply_url)
            .json(&serde_json::json!({
                "launch_profile": profile.launch_profile,
                "provider": {
                    "api": profile.provider.api,
                    "model": profile.provider.model,
                    "gateway_base_url": profile.provider.gateway_base_url,
                    "api_key": api_key,
                }
            }))
            .send()
            .await
            .map_err(|error| AgentRegistryError::RequestFailed(error.to_string()))?;
        if response.status() != StatusCode::OK {
            return Err(AgentRegistryError::BadStatus(response.status()));
        }
        let apply = response
            .json::<SantiConfigApplyResponse>()
            .await
            .map_err(|error| AgentRegistryError::DecodeFailed(error.to_string()))?;

        let active_instance_id = self.active_instance_id().await;
        let launched_pids = self.launched_pid_snapshot().await;
        let snapshot = self
            .probe_configured_instance(instance, &active_instance_id, &launched_pids)
            .await;

        Ok(AgentProfileApplyResponse {
            event_id: action_event_id("profile-apply"),
            instance_id: instance.id.clone(),
            profile_id: profile.id.clone(),
            status: AgentProfileApplyStatus::Applied,
            santi_event_id: apply.event_id,
            config_version: apply.config_version,
            snapshot,
            detail: Some(format!("applied profile {} to {}", profile.id, instance.id)),
        })
    }

    pub async fn probe_instance_provider(
        &self,
        instance_id: &str,
    ) -> Result<SantiProviderProbeFacts, AgentRegistryError> {
        let instance = self
            .find_instance(instance_id)
            .ok_or(AgentRegistryError::NotFound)?;

        self.probe_provider(&instance.provider_probe_url()).await
    }

    async fn launched_pid_snapshot(&self) -> HashMap<String, u32> {
        self.launched_processes
            .read()
            .await
            .iter()
            .map(|(id, child)| (id.clone(), child.id()))
            .collect()
    }

    fn find_instance(&self, instance_id: &str) -> Option<&AgentInstanceConfig> {
        self.instances
            .iter()
            .find(|instance| instance.id == instance_id)
    }

    fn find_profile(&self, profile_id: &str) -> Option<&AgentProfileConfig> {
        self.profiles
            .iter()
            .find(|profile| profile.id == profile_id)
    }
}

pub enum AgentRegistryError {
    NotFound,
    ProfileNotFound,
    Unmanaged,
    LaunchUnavailable,
    AlreadyRunning,
    NotRunning,
    SecretMissing(String),
    LaunchFailed(String),
    StopFailed(String),
    RequestFailed(String),
    BadStatus(StatusCode),
    DecodeFailed(String),
}

impl AgentRegistryError {
    pub fn message(&self) -> String {
        match self {
            Self::NotFound => "agent instance not registered".into(),
            Self::ProfileNotFound => "agent profile not registered".into(),
            Self::Unmanaged => "agent instance is attached, not managed".into(),
            Self::LaunchUnavailable => "managed agent instance has no launch command".into(),
            Self::AlreadyRunning => {
                "managed agent instance is already launched by this agents sidecar".into()
            }
            Self::NotRunning => {
                "managed agent instance was not launched by this agents sidecar".into()
            }
            Self::SecretMissing(profile_id) => {
                format!("agent profile {profile_id} has no available API key")
            }
            Self::LaunchFailed(error) => format!("managed Santi launch failed: {error}"),
            Self::StopFailed(error) => format!("managed Santi stop failed: {error}"),
            Self::RequestFailed(error) => format!("santi provider probe request failed: {error}"),
            Self::BadStatus(status) => {
                format!("santi provider probe returned HTTP {status}")
            }
            Self::DecodeFailed(error) => {
                format!("santi provider probe response could not be decoded: {error}")
            }
        }
    }
}

impl From<stim_platform::process::StopProcessResult> for AgentProcessStopFacts {
    fn from(result: stim_platform::process::StopProcessResult) -> Self {
        Self {
            already_stopped: result.already_stopped,
            matched_pids: result.matched_pids,
            stopped_pids: result.stopped_pids,
            forced_pids: result.forced_pids,
            remaining_pids: result.remaining_pids,
        }
    }
}

fn timestamp_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn action_event_id(action: &str) -> String {
    format!("agents-{action}-{}", timestamp_now())
}
