//! SidecarRuntime adoption for the stim Tauri host.
//!
//! At setup time the host:
//! 1. Synchronously binds a TCP listener via `stim_sidecar::runtime::bind`
//!    (using a one-shot tokio runtime).
//! 2. Emits a `stim-sidecar-ready` line on stdout carrying the bound
//!    address as `runtime_endpoint` so stim-dev can record it in the
//!    chain context and route event-trigger calls to it.
//! 3. Spawns a side thread that owns its own multi-threaded tokio
//!    runtime and runs `runtime::serve` for the lifetime of the host.
//!
//! The legacy file-IPC bridge in `inspection::request_handler`
//! continues to run alongside this socket path during the
//! transition window. stim-dev routes inspect verbs through
//! whichever path the chain context exposes; once everyone has
//! migrated, the bridge file path can be deleted.

use std::{future::Future, pin::Pin, thread, time::Duration};

use serde_json::{json, Value};
use stim_sidecar::{
    identity::{
        mode_or_default, namespace_or_default, SidecarMode, SidecarStamp, SIDECAR_MODE_ENV,
        SIDECAR_NAMESPACE_ENV, SOURCE_APP_TAURI,
    },
    ready::SidecarReadyLine,
    runtime::{self, ClosureHandler, EventError, EventResult},
};
use tauri::AppHandle;

const ROLE: &str = "tauri-runtime";

/// Bind, emit ready-line, spawn serve loop. Called from Tauri's
/// `setup` after window + inspection state is in place but before
/// Cocoa main loop takes over the main thread.
pub fn install(app: AppHandle) -> Result<(), String> {
    let bind_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("tokio bind runtime: {error}"))?;
    let (addr, listener) = bind_runtime
        .block_on(runtime::bind())
        .map_err(|error| format!("sidecar bind: {error}"))?;

    let runtime_endpoint = format!("127.0.0.1:{}", addr.port());
    publish_ready_line(&runtime_endpoint)?;

    thread::spawn(move || {
        let serve_runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(error) => {
                eprintln!("stim-tauri sidecar runtime build failed: {error}");
                return;
            }
        };
        let handler = build_handler(app);
        if let Err(error) = serve_runtime.block_on(runtime::serve(listener, handler)) {
            eprintln!("stim-tauri sidecar serve exited: {error}");
        }
    });

    Ok(())
}

fn publish_ready_line(runtime_endpoint: &str) -> Result<(), String> {
    let namespace = namespace_or_default(std::env::var(SIDECAR_NAMESPACE_ENV).ok().as_deref());
    let mode = mode_or_default(
        std::env::var(SIDECAR_MODE_ENV).ok().as_deref(),
        SidecarMode::Dev,
    );
    let stamp = SidecarStamp {
        app: "tauri".into(),
        namespace,
        mode,
        source: SOURCE_APP_TAURI.into(),
    };
    let ready = SidecarReadyLine::new(
        stamp,
        ROLE.into(),
        format!("tauri-{}", std::process::id()),
        None,
        timestamp_now(),
    )
    .with_runtime_endpoint(runtime_endpoint.to_string());
    let line =
        serde_json::to_string(&ready).map_err(|error| format!("ready line serialize: {error}"))?;
    println!("{line}");
    Ok(())
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

type EventFuture = Pin<Box<dyn Future<Output = EventResult> + Send + 'static>>;
type EventFn = Box<dyn Fn(String, Value) -> EventFuture + Send + Sync + 'static>;

fn build_handler(app: AppHandle) -> ClosureHandler<EventFn> {
    let f: EventFn = Box::new(move |verb: String, _payload: Value| {
        let app = app.clone();
        Box::pin(async move {
            match verb.as_str() {
                "host" => Ok(json!(crate::inspection::inspect::inspect_main_window(&app))),
                "agents-runtime" => Ok(json!(crate::agents_runtime::agents_snapshot(&app))),
                "agents-heartbeat" => Ok(json!(crate::agents_runtime::agents_heartbeat(&app))),
                "controller-runtime" => {
                    Ok(json!(crate::controller_runtime::controller_snapshot(&app)))
                }
                "controller-heartbeat" => {
                    Ok(json!(crate::controller_runtime::controller_heartbeat(&app)))
                }
                other => Err(EventError::not_implemented(other)),
            }
        }) as EventFuture
    });
    ClosureHandler::new(f)
}
