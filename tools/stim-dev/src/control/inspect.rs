use stim_shared::inspection::{
    AgentsRuntimeBridgeResponse, InspectResult, RendererProbeRequest, RendererProbeResult,
    RendererProbeSnapshot, ScreenshotResult,
};

use crate::shared::bridge::{
    request_agents_runtime, request_inspect, request_probe, request_screenshot,
};
use std::time::Duration;

pub(crate) fn inspect(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [app, subcommand] if app == "agents" && subcommand == "runtime" => inspect_agents(),
        [app, subcommand] if app == "agents" && subcommand == "instances" => {
            inspect_agents_instances()
        }
        [app, subcommand] if app == "agents" && subcommand == "profiles" => {
            inspect_agents_profiles()
        }
        [app, subcommand, instance_id] if app == "agents" && subcommand == "probe" => {
            inspect_agents_probe(instance_id)
        }
        [app, subcommand, instance_id] if app == "agents" && subcommand == "provider-probe" => {
            inspect_agents_provider_probe(instance_id)
        }
        [app, subcommand] if app == "tauri" && subcommand == "host" => inspect_host(),
        [app, subcommand] if app == "renderer" && subcommand == "landing" => {
            inspect_renderer(RendererProbeRequest::LandingBasics)
        }
        [app, subcommand] if app == "renderer" && subcommand == "messaging" => {
            inspect_renderer(RendererProbeRequest::MessagingState)
        }
        [app, subcommand] if app == "tauri" && subcommand == "screenshot" => {
            inspect_screenshot(None)
        }
        [app, subcommand, label] if app == "tauri" && subcommand == "screenshot" => {
            inspect_screenshot(Some(label.clone()))
        }
        [] | [_] => Err("inspect requires '<app> <subcommand>'; supported leaves: agents runtime, agents instances, agents profiles, agents probe <instance_id>, agents provider-probe <instance_id>, tauri host, tauri screenshot [label], renderer landing, renderer messaging".into()),
        [app, ..] => Err(format!(
            "unsupported inspect leaf under app '{app}'; supported leaves: agents runtime, agents instances, agents profiles, agents probe <instance_id>, agents provider-probe <instance_id>, tauri host, tauri screenshot [label], renderer landing, renderer messaging"
        )),
    }
}

pub(crate) fn require_renderer_landing() -> Result<RendererProbeSnapshot, String> {
    match request_probe(RendererProbeRequest::LandingBasics)? {
        RendererProbeResult::Success { snapshot } => Ok(*snapshot),
        RendererProbeResult::Failure { reason } => {
            Err(format!("renderer landing probe failed: {:?}", reason))
        }
    }
}

fn inspect_host() -> Result<(), String> {
    match request_inspect()? {
        InspectResult::Success { snapshot } => {
            let output = serde_json::to_string_pretty(&snapshot)
                .map_err(|error| format!("failed to serialize inspect snapshot: {error}"))?;
            println!("{output}");
            Ok(())
        }
        InspectResult::Failure { reason } => Err(format!("inspect failed: {:?}", reason)),
    }
}

fn inspect_agents() -> Result<(), String> {
    let response: AgentsRuntimeBridgeResponse = request_agents_runtime(Duration::from_secs(5))?;
    let output = serde_json::to_string_pretty(&response)
        .map_err(|error| format!("failed to serialize agents runtime snapshot: {error}"))?;
    println!("{output}");
    Ok(())
}

fn inspect_agents_instances() -> Result<(), String> {
    print_json(crate::control::get_agents_json("/api/v1/agents/instances")?)
}

fn inspect_agents_profiles() -> Result<(), String> {
    print_json(crate::control::get_agents_json("/api/v1/agents/profiles")?)
}

fn inspect_agents_probe(instance_id: &str) -> Result<(), String> {
    let path = format!(
        "/api/v1/agents/instances/{}/probe",
        percent_encode_path_segment(instance_id)
    );

    print_json(crate::control::post_agents_empty(&path)?)
}

fn inspect_agents_provider_probe(instance_id: &str) -> Result<(), String> {
    let path = format!(
        "/api/v1/agents/instances/{}/provider/probe",
        percent_encode_path_segment(instance_id)
    );

    print_json(crate::control::post_agents_empty(&path)?)
}

fn inspect_renderer(probe: RendererProbeRequest) -> Result<(), String> {
    match request_probe(probe)? {
        RendererProbeResult::Success { snapshot } => {
            let output = serde_json::to_string_pretty(&snapshot).map_err(|error| {
                format!("failed to serialize renderer inspect snapshot: {error}")
            })?;
            println!("{output}");
            Ok(())
        }
        RendererProbeResult::Failure { reason } => {
            Err(format!("renderer inspect failed: {:?}", reason))
        }
    }
}

fn print_json(value: serde_json::Value) -> Result<(), String> {
    let output = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to serialize agents response: {error}"))?;
    println!("{output}");
    Ok(())
}

pub(crate) fn percent_encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn inspect_screenshot(label: Option<String>) -> Result<(), String> {
    match request_screenshot(label)? {
        ScreenshotResult::Success { path, .. } => {
            println!("{path}");
            Ok(())
        }
        ScreenshotResult::Failure { reason } => Err(format!("screenshot failed: {:?}", reason)),
    }
}
