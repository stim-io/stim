//! SidecarRuntime adoption for the standalone controller binary.
//!
//! Mirrors `apps/tauri/src-tauri/src/sidecar_runtime.rs`: bind a
//! TCP listener via `stim_sidecar::runtime::bind`, emit a
//! `stim-sidecar-ready` line carrying the bound address as
//! `runtime_endpoint`, then spawn a side thread that runs
//! `runtime::serve` on its own multi-threaded tokio runtime.
//!
//! Exposes `controller-runtime` and `controller-heartbeat` as
//! event verbs sourced directly from the in-process
//! `ControllerServiceHandle`. Unknown verbs return
//! `not_implemented` per the SidecarRuntime contract.

use std::{future::Future, pin::Pin, thread};

use serde_json::{json, Value};
use stim_sidecar::{
    identity::SidecarStamp,
    ready::SidecarReadyLine,
    runtime::{self, ClosureHandler, EventError, EventResult},
};

use crate::model::ControllerServiceHandle;

const ROLE: &str = "controller-runtime";

pub fn install(stamp: SidecarStamp, handle: ControllerServiceHandle) -> Result<(), String> {
    let bind_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("tokio bind runtime: {error}"))?;
    let (addr, listener) = bind_runtime
        .block_on(runtime::bind())
        .map_err(|error| format!("sidecar bind: {error}"))?;

    let runtime_endpoint = format!("127.0.0.1:{}", addr.port());
    publish_ready_line(stamp, &handle, runtime_endpoint.clone())?;

    let handler_handle = handle;
    thread::spawn(move || {
        let serve_runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(error) => {
                eprintln!("stim-controller sidecar runtime build failed: {error}");
                return;
            }
        };
        let handler = build_handler(handler_handle);
        if let Err(error) = serve_runtime.block_on(runtime::serve(listener, handler)) {
            eprintln!("stim-controller sidecar serve exited: {error}");
        }
    });

    Ok(())
}

fn publish_ready_line(
    stamp: SidecarStamp,
    handle: &ControllerServiceHandle,
    runtime_endpoint: String,
) -> Result<(), String> {
    let snapshot = handle.snapshot();
    let ready = SidecarReadyLine::new(
        stamp,
        ROLE.into(),
        snapshot.instance_id,
        snapshot.http_base_url,
        snapshot.published_at,
    )
    .with_runtime_endpoint(runtime_endpoint);
    let line =
        serde_json::to_string(&ready).map_err(|error| format!("ready line serialize: {error}"))?;
    println!("{line}");
    Ok(())
}

type EventFuture = Pin<Box<dyn Future<Output = EventResult> + Send + 'static>>;
type EventFn = Box<dyn Fn(String, Value) -> EventFuture + Send + Sync + 'static>;

fn build_handler(handle: ControllerServiceHandle) -> ClosureHandler<EventFn> {
    let f: EventFn = Box::new(move |verb: String, _payload: Value| {
        let handle = handle.clone();
        Box::pin(async move {
            match verb.as_str() {
                "controller-runtime" => Ok(json!(handle.snapshot())),
                "controller-heartbeat" => Ok(json!(handle.heartbeat())),
                other => Err(EventError::not_implemented(other)),
            }
        }) as EventFuture
    });
    ClosureHandler::new(f)
}
