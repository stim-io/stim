pub(super) use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub(super) use axum::{
    http::StatusCode, response::IntoResponse, routing::get, routing::post, Json, Router,
};
pub(super) use stim_proto::{
    AcknowledgementResult, MessageEnvelope, ProtocolAcknowledgement, ProtocolSubmission,
    ReplyHandle, ReplySnapshot, ReplyStatus,
};
pub(super) use stim_server::{
    app::build_router as build_stim_server_router, state::AppState as StimServerState,
};
pub(super) use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationReferenceKind, ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};
pub(super) use tungstenite::Message as WebSocketMessage;

pub(super) use crate::client::{
    fetch_santi_conversation_messages, santi_model::SantiSessionMessagesResponse,
};
pub(super) use crate::fetch;

pub(super) use crate::runtime::spawn_local_controller;

pub(super) static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn spawn_test_stim_server() -> String {
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

pub(super) fn spawn_test_santi_server() -> String {
    spawn_santi_fail_server(0)
}

pub(super) fn spawn_santi_fail_server(transient_transcript_failures: usize) -> String {
    spawn_santi_flaky_server(transient_transcript_failures, StatusCode::BAD_GATEWAY)
}

pub(super) fn spawn_santi_flaky_server(
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

pub(super) fn register_test_agent_participant(stim_server_base_url: &str, participant_id: &str) {
    reqwest::blocking::Client::new()
        .put(format!(
            "{stim_server_base_url}/api/v1/agents/instances/local-santi"
        ))
        .json(&serde_json::json!({
            "agent_id": "santi",
            "instance_id": "local-santi",
            "participant_id": participant_id,
            "delivery_endpoint_id": "endpoint-b",
            "label": "Local Santi",
            "agent_kind": "santi",
            "endpoint": "http://127.0.0.1:18081",
            "profile": "test",
            "capabilities": ["santi"],
            "status": "ready",
            "detail": "registered from test"
        }))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
}

pub(super) fn connect_websocket_with_retry(
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

pub(super) fn read_operation_events(
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

pub(super) fn fetch_product_chat_messages(base_url: &str, session_id: &str) -> serde_json::Value {
    reqwest::blocking::get(format!(
        "{base_url}/api/v1/chat/sessions/{session_id}/messages"
    ))
    .unwrap()
    .error_for_status()
    .unwrap()
    .json()
    .unwrap()
}

pub(super) fn http_response_json(response: &str) -> serde_json::Value {
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .expect("http response should contain a json body");
    serde_json::from_str(body).unwrap()
}
