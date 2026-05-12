//! SidecarRuntime adoption for the standalone controller binary.
//!
//! Mirrors `apps/tauri/src-tauri/src/sidecar_runtime.rs`: bind a
//! runtime socket via `stim_sidecar::runtime::bind`, emit a
//! `stim-sidecar-ready` line carrying the bound endpoint as
//! `runtime_endpoint`, then spawn a side thread that runs
//! `runtime::serve` on its own multi-threaded tokio runtime.
//!
//! Exposes concise indexed inspect events as
//! event verbs sourced directly from the in-process
//! `ControllerServiceHandle`. Unknown verbs return
//! `not_implemented` per the SidecarRuntime contract.

use std::{future::Future, pin::Pin, sync::mpsc, thread, time::Duration};

use serde_json::{json, Value};
use stim_sidecar::{
    identity::SidecarStamp,
    ready::SidecarReadyLine,
    runtime::{self, ClosureHandler, EventError, EventResult},
};

use crate::model::ControllerServiceHandle;

const ROLE: &str = "controller-runtime";

pub fn install(stamp: SidecarStamp, handle: ControllerServiceHandle) -> Result<(), String> {
    let (ready_sender, ready_receiver) = mpsc::channel();
    thread::spawn(move || {
        let serve_runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(error) => {
                let _ = ready_sender.send(Err(format!(
                    "stim-controller sidecar runtime build failed: {error}"
                )));
                return;
            }
        };

        let (runtime_endpoint, listener) = match serve_runtime.block_on(runtime::bind()) {
            Ok(bound) => bound,
            Err(error) => {
                let _ = ready_sender.send(Err(format!("sidecar bind: {error}")));
                return;
            }
        };
        if let Err(error) = publish_ready_line(stamp, &handle, runtime_endpoint) {
            let _ = ready_sender.send(Err(error));
            return;
        }
        let _ = ready_sender.send(Ok(()));

        let handler = build_handler(handle);
        if let Err(error) = serve_runtime.block_on(runtime::serve(listener, handler)) {
            eprintln!("stim-controller sidecar serve exited: {error}");
        }
    });

    ready_receiver
        .recv_timeout(Duration::from_secs(10))
        .map_err(|error| format!("controller sidecar ready wait failed: {error}"))?
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
                "capabilities" => Ok(json!({
                    "events": [
                        "capabilities",
                        "runtime.snapshot",
                        "runtime.heartbeat",
                        "accept.messaging",
                        "accept.participant-routing",
                        "accept.tool-activity"
                    ]
                })),
                "runtime.snapshot" => Ok(json!(handle.snapshot())),
                "runtime.heartbeat" => Ok(json!(handle.heartbeat())),
                "accept.messaging" | "accept.participant-routing" | "accept.tool-activity" => {
                    crate::service::run_acceptance_event(handle.http_state(), &verb, _payload)
                        .await
                        .map_err(EventError::internal)
                }
                other => Err(EventError::not_implemented(other)),
            }
        }) as EventFuture
    });
    ClosureHandler::new(f)
}
