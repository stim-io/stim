use std::{env, net::SocketAddr};

use stim_agents::{app::build_router, state::AppState, stim_server::spawn_registration_loop};
use stim_sidecar::{ready::SidecarReadyLine, stamp::read_stamp};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("stim-agents: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();

    match args.first().map(String::as_str) {
        Some("serve") => {
            args.remove(0);
            serve(args).await
        }
        Some("--help") | Some("-h") | Some("help") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unsupported command: {other}")),
    }
}

async fn serve(args: Vec<String>) -> Result<(), String> {
    let stamp = read_stamp(&args).map_err(|error| format!("invalid sidecar stamp: {error}"))?;
    let state = AppState::from_env(Some(&stamp.namespace))?;
    spawn_registration_loop(state.clone());
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(bind_addr())
        .await
        .map_err(|error| format!("failed to bind agents listener: {error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read agents listener addr: {error}"))?;
    let ready_at = timestamp_now();
    let ready_line = SidecarReadyLine::new(
        stamp,
        "agents-runtime".into(),
        format!("agents-{}", local_addr.port()),
        Some(format!("http://{local_addr}")),
        ready_at,
    );
    let output = serde_json::to_string(&ready_line)
        .map_err(|error| format!("failed to serialize ready line: {error}"))?;

    println!("{output}");

    axum::serve(listener, app)
        .await
        .map_err(|error| format!("agents HTTP server stopped: {error}"))
}

fn bind_addr() -> SocketAddr {
    env::var("STIM_AGENTS_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:0".into())
        .parse()
        .expect("STIM_AGENTS_BIND_ADDR must be a socket address")
}

fn timestamp_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn print_help() {
    println!(
        "stim-agents commands:\n  serve <stamp args>  run stamped agents sidecar HTTP runtime"
    );
}
