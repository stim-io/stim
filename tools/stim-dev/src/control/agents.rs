use std::{
    fs::File,
    io::{BufRead, BufReader},
    time::Duration,
};

use stim_sidecar::{layout::SidecarLayout, ready::SidecarReadyLine};

use crate::shared::bridge::request_agents_runtime;

use super::namespace::current_namespace;

const AGENTS_ENDPOINT_ENV: &str = "STIM_AGENTS_ENDPOINT";

pub(crate) fn agents(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [subcommand, instance_id] if subcommand == "select" => print_agents_json(put_agents_json(
            "/api/v1/agents/selection",
            serde_json::json!({ "instance_id": instance_id }),
        )?),
        [subcommand, instance_id] if subcommand == "launch" => print_agents_json(
            post_agents_empty(&format!(
                "/api/v1/agents/instances/{}/launch",
                percent_encode_path_segment(instance_id)
            ))?,
        ),
        [subcommand, instance_id] if subcommand == "stop" => print_agents_json(
            post_agents_empty(&format!(
                "/api/v1/agents/instances/{}/stop",
                percent_encode_path_segment(instance_id)
            ))?,
        ),
        [subcommand, instance_id, profile_id] if subcommand == "apply-profile" => {
            print_agents_json(post_agents_json(
                &format!(
                    "/api/v1/agents/instances/{}/profiles/apply",
                    percent_encode_path_segment(instance_id)
                ),
                serde_json::json!({ "profile_id": profile_id }),
            )?)
        }
        [] | [_] => Err(
            "agents requires a supported management leaf; supported leaves: select <instance_id>, launch <instance_id>, stop <instance_id>, apply-profile <instance_id> <profile_id>"
                .into(),
        ),
        [subcommand, ..] => Err(format!(
            "unsupported agents leaf: {subcommand}; supported leaves: select <instance_id>, launch <instance_id>, stop <instance_id>, apply-profile <instance_id> <profile_id>"
        )),
    }
}

pub(crate) fn get_agents_json(path: &str) -> Result<serde_json::Value, String> {
    request_agents_json(reqwest::Method::GET, path, None)
}

pub(crate) fn post_agents_empty(path: &str) -> Result<serde_json::Value, String> {
    request_agents_json(reqwest::Method::POST, path, None)
}

pub(crate) fn post_agents_json(
    path: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    request_agents_json(reqwest::Method::POST, path, Some(body))
}

pub(crate) fn put_agents_json(
    path: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    request_agents_json(reqwest::Method::PUT, path, Some(body))
}

fn request_agents_json(
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let base_url = resolve_agents_url()?;
    let url = agents_url(&base_url, path);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("failed to build agents HTTP client: {error}"))?;
    let mut request = client.request(method.clone(), &url);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .map_err(|error| format!("agents HTTP {method} {url} failed: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .unwrap_or_else(|error| format!("<failed to read body: {error}>"));
        return Err(format!(
            "agents HTTP {method} {url} returned {status}: {body}"
        ));
    }

    response
        .json::<serde_json::Value>()
        .map_err(|error| format!("agents HTTP {method} {url} returned invalid JSON: {error}"))
}

fn resolve_agents_url() -> Result<String, String> {
    if let Some(endpoint) = non_empty_env(AGENTS_ENDPOINT_ENV) {
        return Ok(endpoint);
    }

    if let Ok(response) = request_agents_runtime(Duration::from_millis(700)) {
        if let Some(endpoint) = response.snapshot.http_base_url {
            return Ok(endpoint);
        }
    }

    if let Some(endpoint) = read_agents_ready_url()? {
        return Ok(endpoint);
    }

    Err(
        "agents HTTP endpoint unavailable; run 'stim-dev restart agents' or start the full app loop"
            .into(),
    )
}

fn read_agents_ready_url() -> Result<Option<String>, String> {
    let namespace = current_namespace();
    let layout = SidecarLayout::new(stim_platform::paths::dev_root(), Some(&namespace));
    let log_path = layout.app_log_path("agents");
    let file = match File::open(&log_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to open {}: {error}", log_path.display())),
    };
    let mut latest = None;

    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;
        let Ok(ready) = serde_json::from_str::<SidecarReadyLine>(line.trim()) else {
            continue;
        };
        if ready.is_ready_line()
            && ready.stamp.app == "agents"
            && ready.stamp.namespace == namespace
            && ready.role == "agents-runtime"
        {
            latest = ready.endpoint;
        }
    }

    Ok(latest)
}

pub(crate) fn agents_url(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

pub(crate) fn percent_encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn print_agents_json(value: serde_json::Value) -> Result<(), String> {
    let output = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to serialize agents response: {error}"))?;
    println!("{output}");
    Ok(())
}
