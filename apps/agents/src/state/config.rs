use std::{
    collections::{BTreeMap, HashSet},
    env,
};

use serde::Deserialize;

use super::model::{AgentInstanceConfig, AgentProfileConfig, AgentProfileProviderConfig};

const INSTANCES_JSON_ENV: &str = "STIM_AGENTS_SANTI_INSTANCES_JSON";
const PROFILES_JSON_ENV: &str = "STIM_AGENTS_SANTI_PROFILES_JSON";

#[derive(Clone, Debug)]
pub struct SantiInstanceConfig {
    pub agent_id: Option<String>,
    pub participant_id: Option<String>,
    pub delivery_endpoint_id: Option<String>,
    pub id: String,
    pub label: String,
    pub endpoint: String,
    pub profile: Option<String>,
    pub managed: bool,
    pub launch: Option<SantiLaunchConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SantiLaunchConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct SantiProfileConfig {
    pub id: String,
    pub label: String,
    pub launch_profile: String,
    pub provider: SantiProfileProviderConfig,
}

#[derive(Clone, Debug)]
pub struct SantiProfileProviderConfig {
    pub api: String,
    pub model: String,
    pub gateway_base_url: String,
    pub api_key: SantiProfileSecretConfig,
}

#[derive(Clone, Debug)]
pub enum SantiProfileSecretConfig {
    Value(String),
    Env(String),
}

#[derive(Debug, Deserialize)]
struct SantiInstanceEnvConfig {
    agent_id: Option<String>,
    participant_id: Option<String>,
    delivery_endpoint_id: Option<String>,
    id: String,
    endpoint: String,
    label: Option<String>,
    profile: Option<String>,
    managed: Option<bool>,
    launch: Option<SantiLaunchConfig>,
}

#[derive(Debug, Deserialize)]
struct SantiProfileEnvConfig {
    id: String,
    label: Option<String>,
    launch_profile: Option<String>,
    provider: SantiProfileProviderEnvConfig,
}

#[derive(Debug, Deserialize)]
struct SantiProfileProviderEnvConfig {
    api: String,
    model: String,
    gateway_base_url: String,
    api_key: Option<String>,
    api_key_env: Option<String>,
}

pub(super) fn configured_santi_instances(
    namespace: &str,
) -> Result<Vec<SantiInstanceConfig>, String> {
    if let Some(raw) = non_empty_env(INSTANCES_JSON_ENV) {
        let parsed = serde_json::from_str::<Vec<SantiInstanceEnvConfig>>(&raw)
            .map_err(|error| format!("{INSTANCES_JSON_ENV} must be a JSON array: {error}"))?;

        return Ok(parsed
            .into_iter()
            .map(|instance| SantiInstanceConfig {
                agent_id: instance.agent_id,
                participant_id: instance.participant_id,
                delivery_endpoint_id: instance.delivery_endpoint_id,
                label: instance.label.unwrap_or_else(|| instance.id.clone()),
                id: instance.id,
                endpoint: instance.endpoint,
                profile: instance.profile,
                managed: instance.managed.unwrap_or(false),
                launch: instance.launch,
            })
            .collect());
    }

    let launch = configured_single_instance_launch()?;
    Ok(vec![SantiInstanceConfig {
        agent_id: non_empty_env("STIM_AGENTS_SANTI_AGENT_ID"),
        participant_id: non_empty_env("STIM_AGENTS_SANTI_PARTICIPANT_ID"),
        delivery_endpoint_id: non_empty_env("STIM_AGENTS_SANTI_DELIVERY_ENDPOINT_ID"),
        id: non_empty_env("STIM_AGENTS_SANTI_INSTANCE_ID").unwrap_or_else(|| "local-santi".into()),
        label: non_empty_env("STIM_AGENTS_SANTI_LABEL").unwrap_or_else(|| "Local Santi".into()),
        endpoint: non_empty_env("STIM_AGENTS_SANTI_BASE_URL")
            .or_else(|| non_empty_env("SANTI_BASE_URL"))
            .unwrap_or_else(|| "http://127.0.0.1:18081".into()),
        profile: non_empty_env("STIM_AGENTS_SANTI_PROFILE")
            .or_else(|| non_empty_env("SANTI_LAUNCH_PROFILE"))
            .or_else(|| Some(namespace.to_string())),
        managed: parse_bool_env("STIM_AGENTS_SANTI_MANAGED")?.unwrap_or(launch.is_some()),
        launch,
    }])
}

pub(super) fn configured_santi_profiles() -> Result<Vec<SantiProfileConfig>, String> {
    if let Some(raw) = non_empty_env(PROFILES_JSON_ENV) {
        let parsed = serde_json::from_str::<Vec<SantiProfileEnvConfig>>(&raw)
            .map_err(|error| format!("{PROFILES_JSON_ENV} must be a JSON array: {error}"))?;

        return parsed
            .into_iter()
            .map(|profile| {
                let id = profile.id;
                Ok(SantiProfileConfig {
                    label: profile.label.unwrap_or_else(|| id.clone()),
                    launch_profile: profile.launch_profile.unwrap_or_else(|| id.clone()),
                    id,
                    provider: SantiProfileProviderConfig {
                        api: profile.provider.api,
                        model: profile.provider.model,
                        gateway_base_url: profile.provider.gateway_base_url,
                        api_key: profile_secret_config(
                            profile.provider.api_key,
                            profile.provider.api_key_env,
                        )?,
                    },
                })
            })
            .collect();
    }

    Ok(default_santi_profiles())
}

pub(super) fn default_santi_profiles() -> Vec<SantiProfileConfig> {
    vec![
        SantiProfileConfig {
            id: "local".into(),
            label: "Local relay".into(),
            launch_profile: "local-foreground".into(),
            provider: SantiProfileProviderConfig {
                api: "responses".into(),
                model: "gpt-5.4".into(),
                gateway_base_url: "http://127.0.0.1:18082/openai/v1".into(),
                api_key: SantiProfileSecretConfig::Value("codex-local-dev".into()),
            },
        },
        SantiProfileConfig {
            id: "deepseek".into(),
            label: "DeepSeek".into(),
            launch_profile: "local-foreground-deepseek".into(),
            provider: SantiProfileProviderConfig {
                api: "chat-completions".into(),
                model: non_empty_env("DEEPSEEK_MODEL").unwrap_or_else(|| "deepseek-chat".into()),
                gateway_base_url: non_empty_env("DEEPSEEK_BASE_URL")
                    .unwrap_or_else(|| "https://api.deepseek.com".into()),
                api_key: SantiProfileSecretConfig::Env("DEEPSEEK_API_KEY".into()),
            },
        },
    ]
}

pub(super) fn validate_instances(
    namespace: &str,
    instances: Vec<SantiInstanceConfig>,
) -> Result<Vec<AgentInstanceConfig>, String> {
    if instances.is_empty() {
        return Err("at least one Santi instance must be configured".into());
    }

    let mut ids = HashSet::new();
    let mut validated = Vec::with_capacity(instances.len());
    for instance in instances {
        let id = required_field("id", instance.id)?;
        let agent_id = instance
            .agent_id
            .map(|agent_id| required_field("agent_id", agent_id))
            .transpose()?
            .unwrap_or_else(|| id.clone());
        let participant_id = instance
            .participant_id
            .map(|participant_id| required_field("participant_id", participant_id))
            .transpose()?
            .unwrap_or_else(|| agent_id.clone());
        let delivery_endpoint_id = instance
            .delivery_endpoint_id
            .map(|endpoint_id| required_field("delivery_endpoint_id", endpoint_id))
            .transpose()?
            .unwrap_or_else(|| default_delivery_endpoint_id(&id));
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate Santi instance id: {id}"));
        }
        validated.push(AgentInstanceConfig {
            id,
            agent_id,
            participant_id,
            delivery_endpoint_id,
            label: required_field("label", instance.label)?,
            namespace: namespace.to_string(),
            endpoint: required_field("endpoint", instance.endpoint)?,
            profile: instance
                .profile
                .map(|profile| profile.trim().to_string())
                .filter(|profile| !profile.is_empty()),
            managed: instance.managed,
            launch: instance.launch.map(validate_launch_config).transpose()?,
        });
    }

    Ok(validated)
}

pub(super) fn validate_profiles(
    profiles: Vec<SantiProfileConfig>,
) -> Result<Vec<AgentProfileConfig>, String> {
    if profiles.is_empty() {
        return Err("at least one Santi profile must be configured".into());
    }

    let mut ids = HashSet::new();
    let mut validated = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let id = required_field("profile.id", profile.id)?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate Santi profile id: {id}"));
        }
        validated.push(AgentProfileConfig {
            id,
            label: required_field("profile.label", profile.label)?,
            launch_profile: required_field("profile.launch_profile", profile.launch_profile)?,
            provider: AgentProfileProviderConfig {
                api: required_field("profile.provider.api", profile.provider.api)?,
                model: required_field("profile.provider.model", profile.provider.model)?,
                gateway_base_url: required_field(
                    "profile.provider.gateway_base_url",
                    profile.provider.gateway_base_url,
                )?,
                api_key: profile.provider.api_key,
            },
        });
    }

    Ok(validated)
}

fn validate_launch_config(launch: SantiLaunchConfig) -> Result<SantiLaunchConfig, String> {
    let command = required_field("launch.command", launch.command)?;
    let cwd = launch
        .cwd
        .map(|cwd| cwd.trim().to_string())
        .filter(|cwd| !cwd.is_empty());
    Ok(SantiLaunchConfig {
        command,
        args: launch.args,
        cwd,
        env: launch.env,
    })
}

fn configured_single_instance_launch() -> Result<Option<SantiLaunchConfig>, String> {
    let Some(command) = non_empty_env("STIM_AGENTS_SANTI_LAUNCH_COMMAND") else {
        return Ok(None);
    };
    Ok(Some(SantiLaunchConfig {
        command,
        args: parse_json_env("STIM_AGENTS_SANTI_LAUNCH_ARGS_JSON")?.unwrap_or_default(),
        cwd: non_empty_env("STIM_AGENTS_SANTI_LAUNCH_CWD"),
        env: parse_json_env("STIM_AGENTS_SANTI_LAUNCH_ENV_JSON")?.unwrap_or_default(),
    }))
}

fn parse_json_env<T: serde::de::DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    let Some(raw) = non_empty_env(key) else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|error| format!("{key} contains invalid JSON: {error}"))
}

fn parse_bool_env(key: &str) -> Result<Option<bool>, String> {
    let Some(raw) = non_empty_env(key) else {
        return Ok(None);
    };
    match raw.as_str() {
        "1" | "true" | "TRUE" | "yes" | "YES" => Ok(Some(true)),
        "0" | "false" | "FALSE" | "no" | "NO" => Ok(Some(false)),
        _ => Err(format!("{key} must be true or false")),
    }
}

fn profile_secret_config(
    api_key: Option<String>,
    api_key_env: Option<String>,
) -> Result<SantiProfileSecretConfig, String> {
    match (api_key, api_key_env) {
        (Some(value), None) => Ok(SantiProfileSecretConfig::Value(required_field(
            "profile.provider.api_key",
            value,
        )?)),
        (None, Some(key)) => Ok(SantiProfileSecretConfig::Env(required_field(
            "profile.provider.api_key_env",
            key,
        )?)),
        (Some(_), Some(_)) => {
            Err("profile provider must set only one of api_key or api_key_env".into())
        }
        (None, None) => Err("profile provider must set api_key_env or api_key".into()),
    }
}

fn required_field(name: &str, value: String) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(format!("Santi instance {name} must not be empty"))
    } else {
        Ok(value)
    }
}

fn default_delivery_endpoint_id(instance_id: &str) -> String {
    if instance_id == "local-santi" {
        "endpoint-b".into()
    } else {
        format!("santi-{instance_id}")
    }
}

pub(super) fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
