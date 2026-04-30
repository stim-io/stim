use std::{thread, time::Duration};

const DEFAULT_COMPOSE_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_COMPOSE_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";

#[derive(Debug, Clone, Copy)]
pub(crate) struct TargetResolution {
    source: &'static str,
    env_var: &'static str,
}

impl TargetResolution {
    pub(crate) fn describe(self, base_url: &str) -> String {
        match self.source {
            "env-override" => {
                format!("env-override via {} -> {}", self.env_var, base_url)
            }
            "compose-default" => {
                format!("compose-default via {} -> {}", self.env_var, base_url)
            }
            _ => format!("{} -> {}", self.source, base_url),
        }
    }
}

pub(crate) fn resolve_stim_server_base_url() -> Result<(String, TargetResolution), String> {
    if let Ok(base_url) = std::env::var("STIM_SERVER_BASE_URL") {
        wait_for_health(&base_url)?;
        return Ok((
            base_url,
            TargetResolution {
                source: "env-override",
                env_var: "STIM_SERVER_BASE_URL",
            },
        ));
    }

    if wait_for_health(DEFAULT_COMPOSE_STIM_SERVER_BASE_URL).is_ok() {
        return Ok((
            DEFAULT_COMPOSE_STIM_SERVER_BASE_URL.into(),
            TargetResolution {
                source: "compose-default",
                env_var: "STIM_SERVER_BASE_URL",
            },
        ));
    }

    Err(format!(
        "stim-server unavailable: set STIM_SERVER_BASE_URL or start docker-compose service at {}",
        DEFAULT_COMPOSE_STIM_SERVER_BASE_URL
    ))
}

pub(crate) fn resolve_santi_base_url() -> Result<(String, TargetResolution), String> {
    if let Ok(base_url) = std::env::var("SANTI_BASE_URL") {
        wait_for_health(&base_url)?;
        return Ok((
            base_url,
            TargetResolution {
                source: "env-override",
                env_var: "SANTI_BASE_URL",
            },
        ));
    }

    if wait_for_health(DEFAULT_COMPOSE_SANTI_BASE_URL).is_ok() {
        return Ok((
            DEFAULT_COMPOSE_SANTI_BASE_URL.into(),
            TargetResolution {
                source: "compose-default",
                env_var: "SANTI_BASE_URL",
            },
        ));
    }

    Err(format!(
        "santi unavailable: set SANTI_BASE_URL or start docker-compose service at {}",
        DEFAULT_COMPOSE_SANTI_BASE_URL
    ))
}

fn wait_for_health(base_url: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();

    for _ in 0..20 {
        match client.get(format!("{base_url}/api/v1/health")).send() {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    Err("stim-server health never became ready".into())
}
