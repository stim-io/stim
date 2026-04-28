use std::{
    env,
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    process::{exit, Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::Router;
use stim_sidecar::{ready::SidecarReadyLine, stamp::read_stamp};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

const READY_TIMEOUT: Duration = Duration::from_secs(120);
const RENDERER_ROLE: &str = "renderer-delivery";

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
    let mut pnpm_args = vec!["dev", "--host", "127.0.0.1", "--port", "1420"];
    if force {
        pnpm_args.push("--force");
    }
    let mut child = Command::new("pnpm")
        .args(pnpm_args)
        .current_dir(stim_shared::paths::renderer_vite_dir())
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to spawn renderer dev server: {error}"))?;

    if let Err(error) = wait_for_tcp_child("127.0.0.1:1420", &mut child, READY_TIMEOUT) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }

    print_ready(stamp, stim_shared::RENDERER_DEV_URL.to_string())?;
    wait_child(child, "renderer dev server")
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

fn wait_for_tcp_child(
    address: &str,
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = SystemTime::now() + timeout;
    let address = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}"))?;

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed checking renderer dev server status: {error}"))?
        {
            return Err(format!(
                "renderer dev server exited before ready with status {status}"
            ));
        }

        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            thread::sleep(Duration::from_millis(500));
            if let Some(status) = child.try_wait().map_err(|error| {
                format!("failed checking renderer dev server status after TCP ready: {error}")
            })? {
                return Err(format!(
                    "renderer dev server exited after TCP ready with status {status}"
                ));
            }
            return Ok(());
        }

        if SystemTime::now() >= deadline {
            return Err(format!("timed out waiting for {address}"));
        }

        thread::sleep(Duration::from_millis(200));
    }
}
