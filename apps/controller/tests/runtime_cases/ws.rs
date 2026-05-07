use super::support::*;

#[test]
fn serves_ws_operation_events() {
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
            participant_id: None,
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
    let resolved_event = events
        .iter()
        .find(|event| event.stage == ControllerOperationStage::DeliveryTargetResolved)
        .expect("delivery target should be resolved before delivery starts");
    assert_eq!(resolved_event.status, ControllerOperationStatus::Completed);
    assert_eq!(
        resolved_event.detail.as_deref(),
        Some("using direct endpoint endpoint-b")
    );
    assert!(resolved_event.references.iter().any(|reference| {
        reference.reference_kind == ControllerOperationReferenceKind::DeliveryEndpoint
            && reference.endpoint_id.as_deref() == Some("endpoint-b")
    }));
    assert!(events
        .iter()
        .any(|event| event.stage == ControllerOperationStage::DeliveryStarted));
    let chunk_events = events
        .iter()
        .filter(|event| event.stage == ControllerOperationStage::MessageChunkAppended)
        .collect::<Vec<_>>();
    let chunk_text = chunk_events
        .iter()
        .filter_map(|event| event.message_delta.as_ref())
        .map(|delta| delta.text.as_str())
        .collect::<String>();
    assert!(!chunk_text.is_empty());
    assert!("hello from mock santi".starts_with(&chunk_text));
    assert_eq!(terminal.stage, ControllerOperationStage::OperationCompleted);
    assert_eq!(terminal.status, ControllerOperationStatus::Completed);
    assert!(terminal.references.iter().any(|reference| {
        reference.reference_kind == ControllerOperationReferenceKind::ProtocolEnvelope
            && reference.message_id.as_deref().is_some()
            && reference.envelope_id.as_deref().is_some()
            && reference.reply_id.as_deref().is_some()
    }));
    assert!(terminal.references.iter().any(|reference| {
        reference.reference_kind == ControllerOperationReferenceKind::ProductMessageFact
            && reference.ledger_id.as_deref() == Some("stim-server.product-chat")
            && reference.message_id.as_deref().is_some()
    }));
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

    let product_messages = fetch_product_chat_messages(
        &stim_server_base_url,
        terminal.conversation_id.as_deref().unwrap(),
    );
    assert_eq!(
        product_messages
            .pointer("/messages/0/message_kind")
            .and_then(serde_json::Value::as_str),
        Some("user")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/0/state")
            .and_then(serde_json::Value::as_str),
        Some("completed")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/0/text")
            .and_then(serde_json::Value::as_str),
        Some("hello over websocket")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/1/message_kind")
            .and_then(serde_json::Value::as_str),
        Some("assistant")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/1/state")
            .and_then(serde_json::Value::as_str),
        Some("completed")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/1/text")
            .and_then(serde_json::Value::as_str),
        Some("hello from mock santi")
    );
    let product_chunk_text = product_messages
        .pointer("/messages/1/chunks")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|chunk| chunk.pointer("/text").and_then(serde_json::Value::as_str))
        .collect::<String>();
    assert_eq!(product_chunk_text, "hello from mock santi");
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn resolves_ws_delivery_target() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    register_test_agent_participant(&stim_server_base_url, "santi");
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-ws-participant")).unwrap();
    let snapshot = handle.snapshot();
    let http_base_url = snapshot.http_base_url.unwrap();
    let ws_url = format!(
        "{}/api/v1/controller/operations/ws",
        http_base_url.replacen("http://", "ws://", 1)
    );

    let mut socket = connect_websocket_with_retry(&ws_url);
    let command = ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: "op-ws-participant-1".into(),
        correlation_id: "corr-ws-participant-1".into(),
        command: ControllerOperationCommand::SendText {
            text: "hello over participant websocket".into(),
            target_endpoint_id: "not-used".into(),
            participant_id: Some("santi".into()),
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
    let resolved_event = events
        .iter()
        .find(|event| event.stage == ControllerOperationStage::DeliveryTargetResolved)
        .expect("participant delivery target should be resolved");
    let delivery_started_event = events
        .iter()
        .find(|event| event.stage == ControllerOperationStage::DeliveryStarted)
        .expect("delivery should start after target resolution");

    assert_eq!(
        resolved_event.detail.as_deref(),
        Some("resolved participant santi to endpoint endpoint-b")
    );
    assert!(resolved_event.references.iter().any(|reference| {
        reference.reference_kind == ControllerOperationReferenceKind::Participant
            && reference.participant_id.as_deref() == Some("santi")
            && reference.endpoint_id.as_deref() == Some("endpoint-b")
    }));
    assert_eq!(
        delivery_started_event.detail.as_deref(),
        Some("sending text to endpoint-b")
    );
    assert!(delivery_started_event.references.iter().any(|reference| {
        reference.reference_kind == ControllerOperationReferenceKind::DeliveryEndpoint
            && reference.participant_id.as_deref() == Some("santi")
            && reference.endpoint_id.as_deref() == Some("endpoint-b")
    }));
    assert_eq!(terminal.stage, ControllerOperationStage::OperationCompleted);
    assert_eq!(terminal.status, ControllerOperationStatus::Completed);

    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}
