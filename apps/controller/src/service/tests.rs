use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
    time::Duration,
};

use axum::{routing::get, routing::post, Json, Router};
use stim_proto::{
    AcknowledgementResult, MessageEnvelope, ProtocolAcknowledgement, ProtocolSubmission,
    ReplyHandle, ReplySnapshot, ReplyStatus,
};
use stim_server::{
    app::build_router as build_stim_server_router, state::AppState as StimServerState,
};

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
    let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = std_listener.local_addr().unwrap();
    std_listener.set_nonblocking(true).unwrap();

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
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
