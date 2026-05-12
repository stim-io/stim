//! SidecarRuntime adoption for the stim Tauri host.
//!
//! At setup time the host:
//! 1. Synchronously binds a runtime socket via `stim_sidecar::runtime::bind`
//!    (using a one-shot tokio runtime).
//! 2. Emits a `stim-sidecar-ready` line on stdout carrying the bound
//!    endpoint as `runtime_endpoint` so the external `sidecar` CLI can
//!    route provider-owned inspect events to it.
//! 3. Spawns a side thread that owns its own multi-threaded tokio
//!    runtime and runs `runtime::serve` for the lifetime of the host.
//!
//! The legacy file-IPC bridge in `inspection::request_handler`
//! continues to run alongside this socket path during the
//! transition window. The external `sidecar` path routes inspect
//! verbs through SidecarRuntime; once everyone has migrated, the
//! bridge file path can be deleted.

use std::{future::Future, pin::Pin, sync::mpsc, thread, time::Duration};

use serde_json::{json, Value};
use stim_sidecar::{
    identity::{
        mode_or_default, namespace_or_default, SidecarMode, SidecarStamp, SIDECAR_MODE_ENV,
        SIDECAR_NAMESPACE_ENV, SOURCE_APP_TAURI,
    },
    ready::SidecarReadyLine,
    runtime::{self, ClosureHandler, EventError, EventResult},
};
use tauri::{AppHandle, Manager};

use stim_shared::inspection::{
    InspectBridgeRequest, InspectBridgeResponse, RendererActionBridgeRequest,
    RendererActionBridgeResponse, RendererActionFailureReason, RendererActionRequest,
    RendererActionResult, RendererActionSnapshot, RendererMessagingSendSnapshot,
    RendererProbeBridgeRequest, RendererProbeBridgeResponse, RendererProbeFailureReason,
    RendererProbeResult, ScreenshotBridgeRequest, ScreenshotBridgeResponse,
};

const ROLE: &str = "tauri-runtime";
const START_TIMEOUT: Duration = Duration::from_secs(10);

/// Bind, emit ready-line, spawn serve loop. Called from Tauri's
/// `setup` after window + inspection state is in place but before
/// Cocoa main loop takes over the main thread.
pub fn install(app: AppHandle) -> Result<(), String> {
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
                    "stim-tauri sidecar runtime build failed: {error}"
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
        if let Err(error) = publish_ready_line(&runtime_endpoint) {
            let _ = ready_sender.send(Err(error));
            return;
        }
        let _ = ready_sender.send(Ok(()));

        let handler = build_handler(app);
        if let Err(error) = serve_runtime.block_on(runtime::serve(listener, handler)) {
            eprintln!("stim-tauri sidecar serve exited: {error}");
        }
    });

    ready_receiver
        .recv_timeout(START_TIMEOUT)
        .map_err(|error| format!("tauri sidecar ready wait failed: {error}"))?
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
    let f: EventFn = Box::new(move |verb: String, payload: Value| {
        let app = app.clone();
        Box::pin(async move {
            match verb.as_str() {
                "capabilities" => Ok(json!({
                    "events": [
                        "capabilities",
                        "host.snapshot",
                        "host.inspect",
                        "host.screenshot",
                        "renderer.probe",
                        "renderer.action",
                        "renderer.smoke.messaging",
                        "renderer.smoke.continuation",
                        "agents.runtime",
                        "agents.heartbeat",
                        "controller.runtime",
                        "controller.heartbeat"
                    ]
                })),
                "host.snapshot" => Ok(json!(crate::inspection::inspect::inspect_main_window(&app))),
                "host.inspect" => inspect(&app, payload),
                "host.screenshot" => screenshot(&app, payload),
                "renderer.probe" => renderer_probe(&app, payload),
                "renderer.action" => renderer_action(&app, payload),
                "renderer.smoke.messaging" => renderer_smoke_messaging(&app, payload),
                "renderer.smoke.continuation" => renderer_smoke_continuation(&app, payload),
                "agents.runtime" => Ok(json!(crate::agents_runtime::agents_snapshot(&app))),
                "agents.heartbeat" => Ok(json!(crate::agents_runtime::agents_heartbeat(&app))),
                "controller.runtime" => {
                    Ok(json!(crate::controller_runtime::controller_snapshot(&app)))
                }
                "controller.heartbeat" => {
                    Ok(json!(crate::controller_runtime::controller_heartbeat(&app)))
                }
                other => Err(EventError::not_implemented(other)),
            }
        }) as EventFuture
    });
    ClosureHandler::new(f)
}

fn inspect(app: &AppHandle, payload: Value) -> EventResult {
    let request = decode_payload::<InspectBridgeRequest>(payload)?;
    Ok(json!(InspectBridgeResponse {
        request_id: request.request_id,
        requested_at: request.requested_at,
        responded_at: crate::inspection::screenshot::timestamp_now(),
        result: crate::inspection::inspect::inspect_main_window(app),
    }))
}

fn screenshot(app: &AppHandle, payload: Value) -> EventResult {
    let request = decode_payload::<ScreenshotBridgeRequest>(payload)?;
    let window = app.get_webview_window("main");
    Ok(json!(ScreenshotBridgeResponse {
        request_id: request.request_id,
        requested_at: request.requested_at,
        responded_at: crate::inspection::screenshot::timestamp_now(),
        result: crate::inspection::screenshot::capture_main_window_screenshot(
            window.as_ref(),
            request.label.as_deref(),
        ),
    }))
}

fn renderer_probe(app: &AppHandle, payload: Value) -> EventResult {
    let request = decode_payload::<RendererProbeBridgeRequest>(payload)?;
    let result = if app.get_webview_window("main").is_none() {
        RendererProbeResult::Failure {
            reason: RendererProbeFailureReason::NoMainWindow,
        }
    } else {
        crate::inspection::renderer_probe::request_renderer_probe(app, &request)
            .map_err(EventError::internal)?
    };
    Ok(json!(RendererProbeBridgeResponse {
        request_id: request.request_id,
        requested_at: request.requested_at,
        responded_at: crate::inspection::screenshot::timestamp_now(),
        result,
    }))
}

fn renderer_action(app: &AppHandle, payload: Value) -> EventResult {
    let request = decode_payload::<RendererActionBridgeRequest>(payload)?;
    let result = if app.get_webview_window("main").is_none() {
        RendererActionResult::Failure {
            reason: RendererActionFailureReason::NoMainWindow,
            detail: None,
        }
    } else {
        crate::inspection::renderer_action::request_renderer_action(app, &request)
            .map_err(EventError::internal)?
    };
    Ok(json!(RendererActionBridgeResponse {
        request_id: request.request_id,
        requested_at: request.requested_at,
        responded_at: crate::inspection::screenshot::timestamp_now(),
        result,
    }))
}

fn renderer_smoke_messaging(app: &AppHandle, payload: Value) -> EventResult {
    let text = payload_text(payload)
        .unwrap_or_else(|| format!("sidecar renderer smoke {}", timestamp_now()));
    let controller = crate::controller_runtime::controller_snapshot(app);
    let new_conversation = request_renderer_action_value(
        app,
        RendererActionRequest::MessagingNewConversation,
        "smoke-new-conversation",
    )?;
    require_action_success(new_conversation.clone(), "new-conversation")?;
    let result = request_renderer_action_value(
        app,
        RendererActionRequest::MessagingSend {
            text: text.clone(),
            target_endpoint_id: Some("endpoint-b".into()),
        },
        "smoke-messaging",
    )?;
    let snapshot = require_messaging_send_snapshot(result.clone(), "messaging")?;
    assert_renderer_message_state(&snapshot.after, &text, None, 1, 1, "messaging")?;
    Ok(json!({
        "state": "passed",
        "controller": controller,
        "submitted_text": text,
        "new_conversation": action_result_json(new_conversation),
        "result": action_result_json(result),
    }))
}

fn renderer_smoke_continuation(app: &AppHandle, payload: Value) -> EventResult {
    const FOLLOWUP: &str =
        "What exact text did I send in my previous user message? Quote it verbatim.";
    let marker_text = payload_text(payload)
        .unwrap_or_else(|| format!("sidecar renderer continuation {}", timestamp_now()));
    let controller = crate::controller_runtime::controller_snapshot(app);
    let new_conversation = request_renderer_action_value(
        app,
        RendererActionRequest::MessagingNewConversation,
        "continuation-new-conversation",
    )?;
    require_action_success(new_conversation.clone(), "new-conversation")?;
    let first_turn = request_renderer_action_value(
        app,
        RendererActionRequest::MessagingSend {
            text: marker_text.clone(),
            target_endpoint_id: Some("endpoint-b".into()),
        },
        "continuation-first-turn",
    )?;
    let first_snapshot = require_messaging_send_snapshot(first_turn.clone(), "first-turn")?;
    assert_renderer_message_state(
        &first_snapshot.after,
        &marker_text,
        None,
        1,
        1,
        "first-turn",
    )?;
    let second_turn = request_renderer_action_value(
        app,
        RendererActionRequest::MessagingSend {
            text: FOLLOWUP.into(),
            target_endpoint_id: Some("endpoint-b".into()),
        },
        "continuation-second-turn",
    )?;
    let second_snapshot = require_messaging_send_snapshot(second_turn.clone(), "second-turn")?;
    assert_renderer_message_state(
        &second_snapshot.after,
        FOLLOWUP,
        Some(&marker_text),
        2,
        2,
        "second-turn",
    )?;
    Ok(json!({
        "state": "passed",
        "controller": controller,
        "marker_text": marker_text,
        "followup_text": FOLLOWUP,
        "new_conversation": action_result_json(new_conversation),
        "first_turn": action_result_json(first_turn),
        "second_turn": action_result_json(second_turn),
    }))
}

fn request_renderer_action_value(
    app: &AppHandle,
    action: RendererActionRequest,
    label: &str,
) -> Result<RendererActionResult, EventError> {
    let requested_at = crate::inspection::screenshot::timestamp_now();
    let request = RendererActionBridgeRequest {
        request_id: format!("{label}-{requested_at}"),
        requested_at,
        action,
    };
    if app.get_webview_window("main").is_none() {
        return Ok(RendererActionResult::Failure {
            reason: RendererActionFailureReason::NoMainWindow,
            detail: None,
        });
    }
    crate::inspection::renderer_action::request_renderer_action(app, &request)
        .map_err(EventError::internal)
}

fn payload_text(payload: Value) -> Option<String> {
    payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn require_action_success(
    result: RendererActionResult,
    label: &str,
) -> Result<RendererActionSnapshot, EventError> {
    match result {
        RendererActionResult::Success { snapshot } => Ok(*snapshot),
        RendererActionResult::Failure { reason, detail } => Err(EventError::internal(format!(
            "renderer {label} action failed: {}{}",
            action_failure_reason_name(reason),
            detail
                .map(|value| format!(" ({value})"))
                .unwrap_or_default()
        ))),
    }
}

fn require_messaging_send_snapshot(
    result: RendererActionResult,
    label: &str,
) -> Result<RendererMessagingSendSnapshot, EventError> {
    match require_action_success(result, label)? {
        RendererActionSnapshot::MessagingSend(snapshot) => Ok(snapshot),
        RendererActionSnapshot::MessagingNewConversation(_) => Err(EventError::internal(format!(
            "renderer {label} action returned new-conversation snapshot, expected messaging-send"
        ))),
    }
}

fn assert_renderer_message_state(
    snapshot: &stim_shared::inspection::RendererMessagingStateSnapshot,
    expected_user_text: &str,
    expected_assistant_fragment: Option<&str>,
    expected_min_user_entries: usize,
    expected_min_assistant_entries: usize,
    label: &str,
) -> Result<(), EventError> {
    if let Some(error) = snapshot.error_message.as_deref() {
        return Err(EventError::internal(format!(
            "{label} renderer reported visible error: {error}"
        )));
    }
    if !snapshot
        .last_user_text
        .as_deref()
        .is_some_and(|text| text.contains(expected_user_text))
    {
        return Err(EventError::internal(format!(
            "{label} last user text did not include submitted text"
        )));
    }
    if snapshot.user_entry_count < expected_min_user_entries {
        return Err(EventError::internal(format!(
            "{label} expected at least {expected_min_user_entries} user entries, got {}",
            snapshot.user_entry_count
        )));
    }
    if snapshot.assistant_entry_count < expected_min_assistant_entries {
        return Err(EventError::internal(format!(
            "{label} expected at least {expected_min_assistant_entries} assistant entries, got {}",
            snapshot.assistant_entry_count
        )));
    }
    if let Some(fragment) = expected_assistant_fragment {
        if !snapshot
            .last_assistant_text
            .as_deref()
            .is_some_and(|text| text.contains(fragment))
        {
            return Err(EventError::internal(format!(
                "{label} assistant text did not include expected fragment"
            )));
        }
    }
    Ok(())
}

fn action_result_json(result: RendererActionResult) -> Value {
    match result {
        RendererActionResult::Success { snapshot } => {
            json!({ "state": "passed", "snapshot": snapshot })
        }
        RendererActionResult::Failure { reason, detail } => json!({
            "state": "failed",
            "reason": action_failure_reason_name(reason),
            "detail": detail,
        }),
    }
}

fn action_failure_reason_name(reason: RendererActionFailureReason) -> &'static str {
    match reason {
        RendererActionFailureReason::NoMainWindow => "no-main-window",
        RendererActionFailureReason::ActionFailed => "action-failed",
        RendererActionFailureReason::ActionTimedOut => "action-timed-out",
    }
}

fn decode_payload<T: serde::de::DeserializeOwned>(payload: Value) -> Result<T, EventError> {
    serde_json::from_value(payload)
        .map_err(|error| EventError::invalid_payload(format!("invalid payload: {error}")))
}
