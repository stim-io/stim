use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use axum::{http::StatusCode, response::IntoResponse, routing::get, routing::post, Json, Router};
use stim_proto::{
    AcknowledgementResult, MessageEnvelope, ProtocolAcknowledgement, ProtocolSubmission,
    ReplyHandle, ReplySnapshot, ReplyStatus,
};
use stim_server::{
    app::build_router as build_stim_server_router, state::AppState as StimServerState,
};
use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};
use tungstenite::Message as WebSocketMessage;

use crate::client::{fetch_santi_conversation_messages, santi_model::SantiSessionMessagesResponse};
use crate::fetch;

use super::spawn_local_controller;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn spawn_test_stim_server() -> String {
    let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            let app = build_stim_server_router(StimServerState::in_memory());
            axum::serve(listener, app).await.unwrap();
        });
    });

    format!("http://{local_addr}")
}

fn spawn_test_santi_server() -> String {
    spawn_test_santi_server_with_transcript_failures(0)
}

fn spawn_test_santi_server_with_transcript_failures(
    transient_transcript_failures: usize,
) -> String {
    spawn_test_santi_server_with_transient_transcript_status(
        transient_transcript_failures,
        StatusCode::BAD_GATEWAY,
    )
}

fn spawn_test_santi_server_with_transient_transcript_status(
    transient_transcript_failures: usize,
    transient_transcript_status: StatusCode,
) -> String {
    let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();
    let transcript_attempts = Arc::new(AtomicUsize::new(0));

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            let transcript_attempts = transcript_attempts.clone();
            let app = Router::new()
                .route("/api/v1/health", get(|| async { Json("ok") }))
                .route(
                    "/api/v1/stim/envelopes",
                    post(|Json(envelope): Json<MessageEnvelope>| async move {
                        Json(ProtocolSubmission {
                            acknowledgement: ProtocolAcknowledgement {
                                ack_envelope_id: format!("ack-{}", envelope.envelope_id),
                                ack_message_id: envelope.message_id.clone(),
                                ack_version: envelope.new_version,
                                ack_result: AcknowledgementResult::Applied,
                                detail: Some(format!(
                                    "santi session {} applied",
                                    envelope.conversation_id
                                )),
                            },
                            reply: matches!(envelope.operation, stim_proto::MessageOperation::Fix)
                                .then(|| ReplyHandle {
                                    reply_id: "reply-1".into(),
                                    conversation_id: envelope.conversation_id,
                                    message_id: envelope.message_id,
                                    status: ReplyStatus::Pending,
                                }),
                        })
                    }),
                )
                .route(
                    "/api/v1/stim/replies/{reply_id}/events",
                    get(|| async move {
                        let body = concat!(
                            r#"data: {"reply_id":"reply-1","sequence":1,"event":{"type":"output_text_delta","delta":"hello from "}}"#,
                            "\n\n",
                            r#"data: {"reply_id":"reply-1","sequence":2,"event":{"type":"output_text_delta","delta":"mock santi"}}"#,
                            "\n\n",
                            r#"data: {"reply_id":"reply-1","sequence":3,"event":{"type":"completed"}}"#,
                            "\n\n",
                            "data: [DONE]",
                            "\n\n"
                        );
                        ([("content-type", "text/event-stream")], body)
                    }),
                )
                .route(
                    "/api/v1/stim/replies/{reply_id}",
                    get(|| async move {
                        Json(ReplySnapshot {
                            reply_id: "reply-1".into(),
                            conversation_id: "conv-1".into(),
                            message_id: "msg-1".into(),
                            status: ReplyStatus::Completed,
                            output_text: "hello from mock santi".into(),
                            error: None,
                        })
                    }),
                )
                .route(
                    "/api/v1/sessions/{session_id}/messages",
                    get(move || {
                        let transcript_attempts = transcript_attempts.clone();
                        async move {
                            if transcript_attempts.fetch_add(1, Ordering::SeqCst)
                                < transient_transcript_failures
                            {
                                return (
                                    transient_transcript_status,
                                    "transient transcript failure",
                                )
                                    .into_response();
                            }

                            Json(serde_json::json!({
                                "messages": [
                                    {
                                        "id": "msg-1",
                                        "actor_type": "account",
                                        "actor_id": "endpoint-a",
                                        "session_seq": 1,
                                        "content_text": "hello from persisted transcript",
                                        "state": "fixed",
                                        "created_at": "2026-04-30T00:00:00Z"
                                    },
                                    {
                                        "id": "msg-2",
                                        "actor_type": "soul",
                                        "actor_id": "soul_default",
                                        "session_seq": 2,
                                        "content_text": "hello from mock santi",
                                        "state": "fixed",
                                        "created_at": "2026-04-30T00:00:01Z"
                                    }
                                ]
                            }))
                            .into_response()
                        }
                    }),
                )
                .route(
                    "/api/v1/sessions/{session_id}/tool-activities",
                    get(|| async move {
                        Json(serde_json::json!({
                            "tool_activities": [
                                {
                                    "tool_call_id": "call-1",
                                    "tool_name": "bash",
                                    "tool_call_seq": 3,
                                    "tool_call_created_at": "2026-04-30T00:00:02Z",
                                    "tool_result_id": "result-1",
                                    "tool_result_seq": 4,
                                    "tool_result_created_at": "2026-04-30T00:00:03Z",
                                    "result_state": "completed",
                                    "exit_code": 0,
                                    "duration_ms": 12,
                                    "stdout_chars": 5,
                                    "stderr_chars": 0,
                                    "output_summary": "bash exit 0; stdout 5 chars; stderr 0 chars"
                                }
                            ]
                        }))
                    }),
                );

            axum::serve(listener, app).await.unwrap();
        });
    });

    format!("http://{local_addr}")
}

#[test]
fn spawned_controller_serves_first_message_roundtrip_over_http() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-http")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let body = r#"{"text":"hello over http","target_endpoint_id":"endpoint-b"}"#;
                let request = format!(
                    "POST /api/v1/messages/roundtrip HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from mock santi"));
    assert!(response.contains("accepted"));
    assert!(response.contains("endpoint-b"));
    let snapshot_detail = handle.snapshot().detail.unwrap_or_default();
    assert!(snapshot_detail.contains("stim-server env-override via STIM_SERVER_BASE_URL ->"));
    assert!(snapshot_detail.contains("target santi env-override via SANTI_BASE_URL ->"));
    assert!(snapshot_detail.contains("last roundtrip ok for endpoint endpoint-b envelope"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn spawned_controller_serves_conversation_transcript_over_http() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/conv-1/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from persisted transcript"));
    assert!(response.contains("hello from mock santi"));
    assert!(response.contains("\"role\":\"user\""));
    assert!(response.contains("\"role\":\"assistant\""));
    assert!(response.contains("\"tool_activities\""));
    assert!(response.contains("bash exit 0; stdout 5 chars; stderr 0 chars"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn spawned_controller_recovers_from_transient_santi_transcript_failure() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server_with_transcript_failures(1);
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript-retry")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/conv-1/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from persisted transcript"));
    assert!(response.contains("hello from mock santi"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn spawned_controller_maps_persistent_santi_transcript_not_found() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url =
        spawn_test_santi_server_with_transient_transcript_status(100, StatusCode::NOT_FOUND);
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript-not-found")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/missing/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("404 Not Found"));
    assert!(response.contains("fetch status failed: HTTP 404 Not Found"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn santi_transcript_fetch_returns_retry_metadata() {
    let santi_base_url = spawn_test_santi_server_with_transcript_failures(1);

    let result = fetch_santi_conversation_messages(&santi_base_url, "conv-1").unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn santi_transcript_fetch_retries_initial_not_found_projection_gap() {
    let santi_base_url =
        spawn_test_santi_server_with_transient_transcript_status(1, StatusCode::NOT_FOUND);

    let result = fetch_santi_conversation_messages(&santi_base_url, "conv-1").unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn fetch_retry_is_disabled_by_default() {
    let santi_base_url = spawn_test_santi_server_with_transcript_failures(1);
    let result = fetch::FetchClient::new(&santi_base_url).get_json::<SantiSessionMessagesResponse>(
        "/api/v1/sessions/conv-1/messages",
        fetch::FetchRequestOptions::default(),
    );

    let error = result.unwrap_err();

    assert_eq!(error.metadata.attempts, 1);
    assert_eq!(error.metadata.retries, 0);
    assert_eq!(error.metadata.last_status, Some(502));
}

#[test]
fn fetch_retry_supports_custom_decision_closure() {
    let santi_base_url = spawn_test_santi_server_with_transcript_failures(1);
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<SantiSessionMessagesResponse>(
            "/api/v1/sessions/conv-1/messages",
            fetch::FetchRequestOptions::default().with_retry(fetch::FetchRetry::custom(
                fetch::FetchRetryPolicy::new(2, 0, 0),
                |context| {
                    if context.attempt == 1
                        && context.method == reqwest::Method::GET
                        && context.path.ends_with("/messages")
                        && context.status == Some(502)
                    {
                        fetch::FetchRetryDecision::RetryAfter(Duration::from_millis(0))
                    } else {
                        fetch::FetchRetryDecision::Fail
                    }
                },
            )),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn fetch_not_found_payload_is_explicit_option() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<serde_json::Value>(
            "/api/v1/missing",
            fetch::FetchRequestOptions::default()
                .with_not_found_payload(|| serde_json::json!({ "state": "missing" })),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.retries, 0);
    assert_eq!(result.metadata.last_status, Some(404));
    assert_eq!(result.payload, serde_json::json!({ "state": "missing" }));
}

#[test]
fn fetch_not_found_payload_applies_after_matching_retry_policy() {
    let santi_base_url =
        spawn_test_santi_server_with_transient_transcript_status(1, StatusCode::NOT_FOUND);
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<serde_json::Value>(
            "/api/v1/sessions/conv-1/messages",
            fetch::FetchRequestOptions::default()
                .with_retry(fetch::FetchRetry::custom(
                    fetch::FetchRetryPolicy::new(2, 0, 0),
                    |context| {
                        if context.status == Some(404) && context.path.ends_with("/messages") {
                            fetch::FetchRetryDecision::Retry
                        } else {
                            fetch::FetchRetryDecision::Fail
                        }
                    },
                ))
                .with_not_found_payload(|| serde_json::json!({ "state": "missing" })),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert!(result.payload.get("messages").is_some());
}

#[test]
fn fetch_options_are_request_local_overrides() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<String>(
            "/api/v1/health",
            fetch::FetchRequestOptions::default()
                .with_timeout(Duration::from_secs(5))
                .with_header(
                    reqwest::header::HeaderName::from_static("x-stim-fetch-test"),
                    reqwest::header::HeaderValue::from_static("1"),
                )
                .with_query_param("probe", "1")
                .with_status_policy(fetch::FetchStatusPolicy::custom(|status| {
                    status == reqwest::StatusCode::OK
                })),
        )
        .unwrap();

    assert_eq!(result.payload, "ok");
    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.last_status, Some(200));
}

#[test]
fn fetch_retry_decision_is_not_called_for_accepted_status() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<String>(
            "/api/v1/health",
            fetch::FetchRequestOptions::default().with_retry(fetch::FetchRetry::custom(
                fetch::FetchRetryPolicy::new(2, 0, 0),
                |_| panic!("retry decision should not run for accepted status"),
            )),
        )
        .unwrap();

    assert_eq!(result.payload, "ok");
    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.retries, 0);
}

#[test]
fn spawned_controller_serves_message_operation_events_over_websocket() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-ws")).unwrap();
    let snapshot = handle.snapshot();
    let http_base_url = snapshot.http_base_url.unwrap();
    let ws_url = format!(
        "{}/api/v1/controller/operations/ws",
        http_base_url.replacen("http://", "ws://", 1)
    );

    let mut socket = connect_websocket_with_retry(&ws_url);
    let command = ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: "op-ws-1".into(),
        correlation_id: "corr-ws-1".into(),
        command: ControllerOperationCommand::SendText {
            text: "hello over websocket".into(),
            target_endpoint_id: "endpoint-b".into(),
            conversation_id: None,
        },
    };
    socket
        .send(WebSocketMessage::Text(
            serde_json::to_string(&command).unwrap().into(),
        ))
        .unwrap();

    let events = read_operation_events(&mut socket);
    let terminal = events.last().unwrap();
    let snapshot = terminal.snapshot.as_ref().unwrap();

    assert!(events
        .iter()
        .any(|event| event.stage == ControllerOperationStage::CommandAccepted));
    assert!(events
        .iter()
        .any(|event| event.stage == ControllerOperationStage::DeliveryStarted));
    assert_eq!(terminal.stage, ControllerOperationStage::OperationCompleted);
    assert_eq!(terminal.status, ControllerOperationStatus::Completed);
    assert_eq!(
        snapshot.final_sent_text.as_deref(),
        Some("hello over websocket")
    );
    assert_eq!(
        snapshot.response_text_source.as_deref(),
        Some("stim_reply_handle")
    );
    assert_eq!(snapshot.user_message_count, 1);
    assert_eq!(snapshot.assistant_message_count, 1);
    assert_eq!(snapshot.tool_activity_count, 1);
    assert_eq!(snapshot.tool_result_count, 1);
    assert_eq!(snapshot.tool_activities[0].tool_name, "bash");
    assert_eq!(
        snapshot.tool_activities[0].output_summary.as_deref(),
        Some("bash exit 0; stdout 5 chars; stderr 0 chars")
    );
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

fn connect_websocket_with_retry(
    ws_url: &str,
) -> tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    for _ in 0..20 {
        match tungstenite::connect(ws_url) {
            Ok((socket, _)) => return socket,
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    panic!("failed to connect controller websocket at {ws_url}");
}

fn read_operation_events(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) -> Vec<ControllerOperationEvent> {
    let mut events = Vec::new();

    loop {
        let message = socket.read().unwrap();
        let WebSocketMessage::Text(text) = message else {
            continue;
        };
        let event = serde_json::from_str::<ControllerOperationEvent>(&text).unwrap();
        let terminal = event.is_terminal();
        events.push(event);
        if terminal {
            return events;
        }
    }
}

#[test]
fn spawned_controller_snapshot_reports_env_override_targets() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };

    let handle = spawn_local_controller(Some("test-detail")).unwrap();
    let snapshot = handle.snapshot();
    let detail = snapshot.detail.unwrap_or_default();

    assert!(detail.contains("stim-server env-override via STIM_SERVER_BASE_URL ->"));
    assert!(detail.contains(&stim_server_base_url));
    assert!(detail.contains("target santi env-override via SANTI_BASE_URL ->"));
    assert!(detail.contains(&santi_base_url));

    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn spawned_controller_exposes_discovery_registry() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-registry")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/debug/registry HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("endpoint-a"));
    assert!(response.contains("endpoint-b"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}
