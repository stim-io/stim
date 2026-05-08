use std::{
    env,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{exit, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::Router;
use regex::Regex;
use stim_sidecar::{ready::SidecarReadyLine, stamp::read_stamp};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

const READY_TIMEOUT: Duration = Duration::from_secs(120);
const RENDERER_ROLE: &str = "renderer-delivery";
const VITE_LOCAL_LINE_PATTERN: &str = r"Local:\s+(http://[^\s]+)";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("stim-renderer: {error}");
        exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) != Some("serve") {
        return Err("expected: serve --dev [--force] <stamp args> | serve --runtime [--asset-root <path>] <stamp args>".into());
    }
    args.remove(0);

    match args.first().map(String::as_str) {
        Some("--dev") => run_dev(args.split_off(1)),
        Some("--runtime") => run_runtime(args.split_off(1)).await,
        _ => Err("serve requires --dev or --runtime".into()),
    }
}

fn run_dev(args: Vec<String>) -> Result<(), String> {
    let mut force = false;
    let stamp_args = args
        .into_iter()
        .filter(|arg| {
            if arg == "--force" {
                force = true;
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>();
    let stamp =
        read_stamp(&stamp_args).map_err(|error| format!("invalid renderer stamp: {error}"))?;
    let mut pnpm_args = vec!["dev", "--host", "127.0.0.1"];
    if force {
        pnpm_args.push("--force");
    }
    let mut child = Command::new("pnpm")
        .args(pnpm_args)
        .current_dir(stim_shared::paths::renderer_vite_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn renderer dev server: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "renderer dev stdout was not piped".to_string())?;
    let endpoint = match observe_vite_endpoint(stdout, READY_TIMEOUT) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };

    print_ready(stamp, endpoint)?;
    wait_child(child, "renderer dev server")
}

/// Compose `stim_sidecar::stdout::extract_endpoint` with a sync
/// line reader to capture the first vite `Local: http://...`
/// line. After the URL is captured, a side thread keeps draining
/// the rest of vite's stdout so its pipe doesn't fill and block
/// the child.
fn observe_vite_endpoint(stdout: ChildStdout, timeout: Duration) -> Result<String, String> {
    let pattern = Regex::new(VITE_LOCAL_LINE_PATTERN)
        .map_err(|error| format!("invalid vite stdout pattern: {error}"))?;
    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut sent = false;
        for line_res in reader.lines() {
            let Ok(line) = line_res else { break };
            if sent {
                continue;
            }
            if let Some(endpoint) = stim_sidecar::stdout::extract_endpoint(&line, &pattern) {
                let _ = tx.send(Ok(endpoint));
                sent = true;
            }
        }
        if !sent {
            let _ = tx.send(Err(
                "renderer dev server closed stdout before logging endpoint".to_string(),
            ));
        }
    });

    rx.recv_timeout(timeout)
        .map_err(|_| "timed out waiting for vite endpoint".to_string())?
}

async fn run_runtime(args: Vec<String>) -> Result<(), String> {
    let mut asset_root: Option<PathBuf> = None;
    let mut stamp_args = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--asset-root" {
            index += 1;
            asset_root = Some(PathBuf::from(
                args.get(index)
                    .ok_or_else(|| "--asset-root requires a path".to_string())?,
            ));
        } else {
            stamp_args.push(args[index].clone());
        }
        index += 1;
    }
    let stamp =
        read_stamp(&stamp_args).map_err(|error| format!("invalid renderer stamp: {error}"))?;
    let asset_root =
        asset_root.unwrap_or_else(|| stim_shared::paths::renderer_vite_dir().join("dist"));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("failed to bind renderer runtime server: {error}"))?;
    let endpoint = format!(
        "http://{}",
        listener
            .local_addr()
            .map_err(|error| format!("failed to read renderer runtime address: {error}"))?
    );
    let app = Router::new().fallback_service(ServeDir::new(asset_root));
    print_ready(stamp, endpoint)?;
    axum::serve(listener, app)
        .await
        .map_err(|error| format!("renderer runtime server failed: {error}"))
}

fn print_ready(
    stamp: stim_sidecar::identity::SidecarStamp,
    endpoint: String,
) -> Result<(), String> {
    let ready = SidecarReadyLine::new(
        stamp,
        RENDERER_ROLE.into(),
        format!("renderer-{}", timestamp_now()),
        Some(endpoint),
        timestamp_now(),
    );
    let output = serde_json::to_string(&ready)
        .map_err(|error| format!("failed to serialize renderer ready line: {error}"))?;
    println!("{output}");
    Ok(())
}

fn wait_child(mut child: std::process::Child, name: &str) -> Result<(), String> {
    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for {name}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{name} exited with status {status}"))
    }
}

fn timestamp_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");
    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}
