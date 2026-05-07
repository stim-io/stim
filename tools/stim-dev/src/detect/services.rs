use std::{env, time::Duration};

use serde::Serialize;

use super::http::http_get_status;

const DEFAULT_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";
const DEFAULT_SANTI_LINK_BASE_URL: &str = "http://127.0.0.1:18082";

pub(crate) const STANDALONE_COMPOSE_HINT: &str =
    "docker compose up -d --build stim-server santi-link";
pub(crate) const LOCAL_SANTI_HINT: &str = "scripts/santi local";

pub(super) fn default_service_probes() -> Vec<ServiceProbe> {
    vec![
        ServiceProbe::check(
            "stim-server",
            Some("STIM_SERVER_BASE_URL"),
            DEFAULT_SERVER_BASE_URL,
            "/api/v1/health",
            "compose-default",
        ),
        ServiceProbe::check(
            "santi",
            Some("SANTI_BASE_URL"),
            DEFAULT_SANTI_BASE_URL,
            "/api/v1/health",
            "local-santi-default",
        ),
        ServiceProbe::check(
            "santi-link",
            None,
            DEFAULT_SANTI_LINK_BASE_URL,
            "/openai/v1/health",
            "compose-default",
        ),
    ]
}

#[derive(Serialize)]
pub(crate) struct ServiceProbe {
    pub(crate) name: &'static str,
    pub(crate) source: &'static str,
    pub(crate) env_var: Option<&'static str>,
    pub(crate) base_url: String,
    pub(crate) health_path: &'static str,
    pub(crate) state: &'static str,
    pub(crate) detail: String,
}

impl ServiceProbe {
    pub(super) fn check(
        name: &'static str,
        env_var: Option<&'static str>,
        default_base_url: &'static str,
        health_path: &'static str,
        default_source: &'static str,
    ) -> Self {
        let env_base_url = env_var
            .and_then(|key| env::var(key).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let source = if env_base_url.is_some() {
            "env-override"
        } else {
            default_source
        };
        let base_url = env_base_url.unwrap_or_else(|| default_base_url.to_string());

        match http_get_status(&base_url, health_path, Duration::from_millis(700)) {
            Ok(status) if (200..300).contains(&status) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "ready",
                detail: format!("health returned HTTP {status}"),
            },
            Ok(status) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "unhealthy",
                detail: format!("health returned HTTP {status}"),
            },
            Err(error) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "unavailable",
                detail: error,
            },
        }
    }

    pub(super) fn is_ready(&self) -> bool {
        self.state == "ready"
    }

    pub(super) fn uses_compose_default(&self) -> bool {
        self.source == "compose-default"
    }

    pub(super) fn uses_local_santi_default(&self) -> bool {
        self.name == "santi" && self.source == "local-santi-default"
    }
}
