use std::{
    env, fs,
    process::{exit, Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use stim_shared::{
    inspection::{
        InspectBridgeRequest, InspectBridgeResponse, InspectResult, RendererProbeBridgeRequest,
        RendererProbeBridgeResponse, RendererProbeRequest, RendererProbeResult,
        ScreenshotBridgeRequest, ScreenshotBridgeResponse, ScreenshotResult,
    },
    paths::{
        inspect_bridge_request_path, inspect_bridge_response_path, renderer_app_dir,
        renderer_probe_bridge_request_path, renderer_probe_bridge_response_path,
        screenshot_bridge_request_path, screenshot_bridge_response_path, tauri_app_dir,
    },
};

#[derive(Clone, Copy)]
enum StartTarget {
    All,
    Renderer,
    Tauri,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("stim-dev-cli: {error}");
        exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "start".to_string());

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "start" => start(parse_start_target(args.next().as_deref())?),
        "inspect" => inspect(),
        "probe" => probe(args.next().as_deref()),
        "screenshot" => screenshot(args.next()),
        other => Err(format!("unsupported command: {other}\n\n{}", help_text())),
    }
}

fn help_text() -> &'static str {
    "stim-dev-cli commands:\n  start [all|renderer|tauri]\n  inspect\n  probe [landing]\n  screenshot [label]\n  help"
}

fn print_help() {
    println!("{}", help_text());
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn create_request_id() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{}", duration.as_secs(), duration.subsec_millis())
}

fn parse_start_target(value: Option<&str>) -> Result<StartTarget, String> {
    match value.unwrap_or("all") {
        "all" => Ok(StartTarget::All),
        "renderer" => Ok(StartTarget::Renderer),
        "tauri" => Ok(StartTarget::Tauri),
        other => Err(format!("unsupported start target: {other}")),
    }
}

fn start(target: StartTarget) -> Result<(), String> {
    match target {
        StartTarget::All | StartTarget::Tauri => run_pnpm(&tauri_app_dir(), &["tauri", "dev"]),
        StartTarget::Renderer => run_pnpm(
            &renderer_app_dir(),
            &[
                "dev",
                "--host",
                stim_shared::RENDERER_DEV_HOST,
                "--port",
                "1420",
            ],
        ),
    }
}

fn screenshot(label: Option<String>) -> Result<(), String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = ScreenshotBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        label,
    };

    let request_path = screenshot_bridge_request_path(&request_id);
    let response_path = screenshot_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create screenshot request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create screenshot response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize screenshot request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write screenshot request: {error}"))?;

    let started = SystemTime::now();
    let timeout = Duration::from_secs(15);

    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for screenshot response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<ScreenshotBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse screenshot response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);

                match response.result {
                    ScreenshotResult::Success { path, .. } => {
                        println!("{path}");
                        return Ok(());
                    }
                    ScreenshotResult::Failure { reason } => {
                        return Err(format!("screenshot failed: {:?}", reason));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn inspect() -> Result<(), String> {
    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = InspectBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
    };

    let request_path = inspect_bridge_request_path(&request_id);
    let response_path = inspect_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create inspect request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create inspect response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize inspect request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write inspect request: {error}"))?;

    let started = SystemTime::now();
    let timeout = Duration::from_secs(15);

    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for inspect response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<InspectBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse inspect response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);

                match response.result {
                    InspectResult::Success { snapshot } => {
                        let output = serde_json::to_string_pretty(&snapshot).map_err(|error| {
                            format!("failed to serialize inspect snapshot: {error}")
                        })?;
                        println!("{output}");
                        return Ok(());
                    }
                    InspectResult::Failure { reason } => {
                        return Err(format!("inspect failed: {:?}", reason));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn probe(target: Option<&str>) -> Result<(), String> {
    let probe = match target.unwrap_or("landing") {
        "landing" => RendererProbeRequest::LandingBasics,
        other => return Err(format!("unsupported probe target: {other}")),
    };

    let request_id = create_request_id();
    let requested_at = timestamp_now();
    let request = RendererProbeBridgeRequest {
        request_id: request_id.clone(),
        requested_at: requested_at.clone(),
        probe,
    };

    let request_path = renderer_probe_bridge_request_path(&request_id);
    let response_path = renderer_probe_bridge_response_path(&request_id);

    if let Some(parent) = request_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create probe request dir: {error}"))?;
    }

    if let Some(parent) = response_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create probe response dir: {error}"))?;
    }

    let request_body = serde_json::to_string_pretty(&request)
        .map_err(|error| format!("failed to serialize probe request: {error}"))?;
    let _ = fs::remove_file(&response_path);
    fs::write(&request_path, format!("{request_body}\n"))
        .map_err(|error| format!("failed to write probe request: {error}"))?;

    let started = SystemTime::now();
    let timeout = Duration::from_secs(15);

    loop {
        if started.elapsed().unwrap_or_default() > timeout {
            let _ = fs::remove_file(&request_path);
            let _ = fs::remove_file(&response_path);
            return Err(format!(
                "timed out waiting for probe response at {}",
                response_path.display()
            ));
        }

        if let Ok(content) = fs::read_to_string(&response_path) {
            let response = serde_json::from_str::<RendererProbeBridgeResponse>(&content)
                .map_err(|error| format!("failed to parse probe response: {error}"))?;

            if response.request_id == request_id && response.requested_at == requested_at {
                let _ = fs::remove_file(&request_path);
                let _ = fs::remove_file(&response_path);

                match response.result {
                    RendererProbeResult::Success { snapshot } => {
                        let output = serde_json::to_string_pretty(&snapshot).map_err(|error| {
                            format!("failed to serialize probe snapshot: {error}")
                        })?;
                        println!("{output}");
                        return Ok(());
                    }
                    RendererProbeResult::Failure { reason } => {
                        return Err(format!("probe failed: {:?}", reason));
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn run_pnpm(workdir: &std::path::Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("pnpm")
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to run pnpm {:?}: {error}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("pnpm {:?} exited with status {status}", args))
    }
}
