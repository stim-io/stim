use std::collections::HashMap;

use reqwest::StatusCode;

use crate::schema::{
    AgentInstanceSnapshot, AgentInstanceState, SantiConfigFacts, SantiProviderProbeFacts,
    SantiProviderProbeState,
};

use super::{
    model::{AgentInstanceConfig, SantiMetaResponse, SnapshotInput},
    timestamp_now, AgentRegistry, AgentRegistryError,
};

impl AgentRegistry {
    pub(super) async fn probe_configured_instance(
        &self,
        instance: &AgentInstanceConfig,
        active_instance_id: &str,
        launched_pids: &HashMap<String, u32>,
    ) -> AgentInstanceSnapshot {
        let health_url = format!("{}/api/v1/health", instance.endpoint.trim_end_matches('/'));
        let meta_url = format!("{}/api/v1/meta", instance.endpoint.trim_end_matches('/'));
        let probed_at = timestamp_now();
        let active = instance.id == active_instance_id;
        let process =
            launched_pids
                .get(&instance.id)
                .copied()
                .map(|pid| crate::schema::AgentProcessFacts {
                    pid,
                    launched_by_agents: true,
                });

        let health = self.client.get(&health_url).send().await;
        let Ok(health_response) = health else {
            return instance.snapshot(SnapshotInput {
                state: AgentInstanceState::Unreachable,
                active,
                process: process.clone(),
                last_probe_at: probed_at,
                service: None,
                config: None,
                provider: None,
                provider_probe: None,
                runtime: None,
                detail: Some("santi health probe failed".into()),
            });
        };

        if health_response.status() != StatusCode::OK {
            return instance.snapshot(SnapshotInput {
                state: AgentInstanceState::Degraded,
                active,
                process: process.clone(),
                last_probe_at: probed_at,
                service: None,
                config: None,
                provider: None,
                provider_probe: None,
                runtime: None,
                detail: Some(format!(
                    "santi health returned HTTP {}",
                    health_response.status()
                )),
            });
        }

        match self.client.get(meta_url).send().await {
            Ok(response) if response.status() == StatusCode::OK => {
                match response.json::<SantiMetaResponse>().await {
                    Ok(meta) => {
                        let service = meta.service_facts();
                        let config = self.probe_current_config(&instance.config_url()).await;
                        let provider_probe = self
                            .probe_provider(&instance.provider_probe_url())
                            .await
                            .ok();
                        let state = match provider_probe.as_ref().map(|probe| &probe.state) {
                            Some(SantiProviderProbeState::Ready) | None => {
                                AgentInstanceState::Ready
                            }
                            Some(
                                SantiProviderProbeState::Degraded
                                | SantiProviderProbeState::Unreachable,
                            ) => AgentInstanceState::Degraded,
                        };
                        let mut detail = format!(
                            "santi {} {}",
                            service.service_name.as_str(),
                            service.mode.as_deref().unwrap_or("mode-unknown")
                        );
                        if let Some(probe) = provider_probe.as_ref() {
                            detail.push_str(&format!("; provider probe {:?}", probe.state));
                        }

                        instance.snapshot(SnapshotInput {
                            state,
                            active,
                            process: process.clone(),
                            last_probe_at: probed_at,
                            service: Some(service),
                            config,
                            provider: meta.provider,
                            provider_probe,
                            runtime: meta.runtime,
                            detail: Some(detail),
                        })
                    }
                    Err(error) => instance.snapshot(SnapshotInput {
                        state: AgentInstanceState::Degraded,
                        active,
                        process: process.clone(),
                        last_probe_at: probed_at,
                        service: None,
                        config: None,
                        provider: None,
                        provider_probe: None,
                        runtime: None,
                        detail: Some(format!("santi meta response could not be decoded: {error}")),
                    }),
                }
            }
            Ok(response) => instance.snapshot(SnapshotInput {
                state: AgentInstanceState::Degraded,
                active,
                process: process.clone(),
                last_probe_at: probed_at,
                service: None,
                config: None,
                provider: None,
                provider_probe: None,
                runtime: None,
                detail: Some(format!("santi meta returned HTTP {}", response.status())),
            }),
            Err(error) => instance.snapshot(SnapshotInput {
                state: AgentInstanceState::Degraded,
                active,
                process,
                last_probe_at: probed_at,
                service: None,
                config: None,
                provider: None,
                provider_probe: None,
                runtime: None,
                detail: Some(format!("santi meta probe failed: {error}")),
            }),
        }
    }

    pub(super) async fn probe_provider(
        &self,
        provider_probe_url: &str,
    ) -> Result<SantiProviderProbeFacts, AgentRegistryError> {
        match self.client.post(provider_probe_url).send().await {
            Ok(response) if response.status() == StatusCode::OK => response
                .json::<SantiProviderProbeFacts>()
                .await
                .map_err(|error| AgentRegistryError::DecodeFailed(error.to_string())),
            Ok(response) => Err(AgentRegistryError::BadStatus(response.status())),
            Err(error) => Err(AgentRegistryError::RequestFailed(error.to_string())),
        }
    }

    async fn probe_current_config(&self, config_url: &str) -> Option<SantiConfigFacts> {
        let response = self.client.get(config_url).send().await.ok()?;
        if response.status() != StatusCode::OK {
            return None;
        }
        response.json::<SantiConfigFacts>().await.ok()
    }
}
