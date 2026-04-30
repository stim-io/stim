use std::{net::TcpListener, thread};

use axum::{routing::get, routing::post, Json, Router};
use stim_proto::{
    AcknowledgementResult, DeliveryReceiptResult, MessageEnvelope, ProtocolAcknowledgement,
    ProtocolSubmission, ReplyHandle, ReplySnapshot, ReplyStatus,
};

use super::messages::parse_acknowledgement;
use super::{
    first_message_roundtrip_with_records, http_santi_discovery_fixture,
    message_roundtrip_with_records, sample_santi_discovery_record,
};

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
                            r#"data: {"reply_id":"reply-1","sequence":1,"event":{"type":"output_text_delta","delta":"real "}}"#,
                            "\n\n",
                            r#"data: {"reply_id":"reply-1","sequence":2,"event":{"type":"output_text_delta","delta":"santi reply"}}"#,
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
                            output_text: "real santi reply".into(),
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
fn controller_runtime_caches_discovery_and_roundtrips_message() {
    let santi_base_url = spawn_test_santi_server();
    let fixture = http_santi_discovery_fixture("default", &santi_base_url);
    let summary = first_message_roundtrip_with_records(
        "http://127.0.0.1:8080",
        "endpoint-b",
        "hello controller",
        fixture.self_discovery.clone(),
        vec![fixture.peer_discovery.clone(), fixture.self_discovery],
    )
    .unwrap();

    assert_eq!(summary.server_base_url, "http://127.0.0.1:8080");
    assert_eq!(summary.endpoint_id, "endpoint-b");
    assert!(summary.conversation_id.starts_with("conv-"));
    assert!(summary.message_id.starts_with("msg-"));
    assert_eq!(summary.listen_address, santi_base_url);
    assert!(summary.envelope_id.starts_with("env-"));
    assert_eq!(
        summary.final_sent_text,
        "hello controller (edited before fix)"
    );
    assert_eq!(summary.final_message_version, 3);
    assert_eq!(summary.response_envelope_id, "reply-1");
    assert_eq!(summary.response_text, "real santi reply");
    assert_eq!(summary.response_text_source, "stim_reply_handle");
    assert_eq!(summary.receipt_result, DeliveryReceiptResult::Accepted);
    let expected_detail = format!("santi session {} applied", summary.conversation_id);
    assert_eq!(
        summary.receipt_detail.as_deref(),
        Some(expected_detail.as_str())
    );
    assert_eq!(summary.lifecycle_trace.len(), 3);
    assert_eq!(summary.lifecycle_trace[0].operation, "create");
    assert_eq!(summary.lifecycle_trace[0].ack_version, 1);
    assert_eq!(summary.lifecycle_trace[1].operation, "patch");
    assert_eq!(summary.lifecycle_trace[1].ack_version, 2);
    assert_eq!(summary.lifecycle_trace[2].operation, "fix");
    assert_eq!(summary.lifecycle_trace[2].ack_version, 3);
    assert_eq!(summary.lifecycle_proof.create_ack_version, 1);
    assert_eq!(summary.lifecycle_proof.patch_ack_version, 2);
    assert_eq!(summary.lifecycle_proof.fix_ack_version, 3);
    assert_eq!(summary.lifecycle_proof.final_message_version, 3);
    assert_eq!(
        summary.lifecycle_proof.expected_final_text,
        summary.final_sent_text
    );
    assert_eq!(
        summary.lifecycle_proof.controller_final_text,
        summary.final_sent_text
    );
    assert!(summary.lifecycle_proof.final_text_matches_expected);
    assert!(summary.lifecycle_proof.version_progression_valid);
    assert_eq!(summary.cached_endpoint_count, 2);
}

#[test]
fn controller_runtime_uses_registry_records_for_selected_endpoint() {
    let santi_base_url = spawn_test_santi_server();
    let fixture = http_santi_discovery_fixture("registry-test", &santi_base_url);
    let summary = first_message_roundtrip_with_records(
        "http://127.0.0.1:43100",
        "endpoint-b",
        "hello registry",
        fixture.self_discovery.clone(),
        vec![fixture.peer_discovery, fixture.self_discovery],
    )
    .unwrap();

    assert_eq!(summary.server_base_url, "http://127.0.0.1:43100");
    assert_eq!(summary.endpoint_id, "endpoint-b");
    assert_eq!(
        summary.final_sent_text,
        "hello registry (edited before fix)"
    );
    assert_eq!(summary.final_message_version, 3);
    assert_eq!(summary.response_text, "real santi reply");
    assert_eq!(summary.response_text_source, "stim_reply_handle");
    assert_eq!(summary.lifecycle_trace.len(), 3);
    assert!(summary.lifecycle_proof.version_progression_valid);
}

#[test]
fn controller_runtime_can_continue_existing_conversation() {
    let santi_base_url = spawn_test_santi_server();
    let fixture = http_santi_discovery_fixture("continue-test", &santi_base_url);

    let first = message_roundtrip_with_records(
        "http://127.0.0.1:8080",
        "endpoint-b",
        "hello first turn",
        Some("conv-shared"),
        fixture.self_discovery.clone(),
        vec![
            fixture.peer_discovery.clone(),
            fixture.self_discovery.clone(),
        ],
    )
    .unwrap();
    let second = message_roundtrip_with_records(
        "http://127.0.0.1:8080",
        "endpoint-b",
        "hello second turn",
        Some("conv-shared"),
        fixture.self_discovery.clone(),
        vec![fixture.peer_discovery, fixture.self_discovery],
    )
    .unwrap();

    assert_eq!(first.conversation_id, "conv-shared");
    assert_eq!(second.conversation_id, "conv-shared");
    assert_ne!(first.message_id, second.message_id);
    assert_eq!(second.response_text, "real santi reply");
}

#[test]
fn santi_discovery_record_uses_http_attach_target() {
    let record = sample_santi_discovery_record("http://127.0.0.1:18081");

    assert_eq!(record.carrier_kind, "http");
    assert_eq!(record.addresses, vec!["http://127.0.0.1:18081"]);
    assert_eq!(
        record.endpoint_declaration.endpoint_kind.as_deref(),
        Some("santi")
    );
}

#[test]
fn acknowledgement_keeps_receipt_fields_without_output_parsing() {
    let parsed = parse_acknowledgement(&ProtocolAcknowledgement {
        ack_envelope_id: "ack-env-1".into(),
        ack_message_id: "msg-1".into(),
        ack_version: 1,
        ack_result: AcknowledgementResult::Applied,
        detail: Some("santi session conv-1 applied".into()),
    });

    assert_eq!(parsed.receipt_result, DeliveryReceiptResult::Accepted);
    assert_eq!(
        parsed.receipt_detail.as_deref(),
        Some("santi session conv-1 applied")
    );
    assert_eq!(parsed.ack_envelope_id, "ack-env-1");
    assert_eq!(parsed.ack_message_id, "msg-1");
    assert_eq!(parsed.ack_version, 1);
}
