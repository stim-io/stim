use stim_shared::inspection::{
    InspectResult, RendererProbeRequest, RendererProbeResult, RendererProbeSnapshot,
    ScreenshotResult,
};

use crate::shared::bridge::{request_inspect, request_probe, request_screenshot};

pub(crate) fn inspect(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
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
        [] | [_] => Err("inspect requires '<app> <subcommand>'; supported leaves: tauri host, tauri screenshot [label], renderer landing, renderer messaging".into()),
        [app, ..] => Err(format!(
            "unsupported inspect leaf under app '{app}'; supported leaves: tauri host, tauri screenshot [label], renderer landing, renderer messaging"
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

fn inspect_screenshot(label: Option<String>) -> Result<(), String> {
    match request_screenshot(label)? {
        ScreenshotResult::Success { path, .. } => {
            println!("{path}");
            Ok(())
        }
        ScreenshotResult::Failure { reason } => Err(format!("screenshot failed: {:?}", reason)),
    }
}
