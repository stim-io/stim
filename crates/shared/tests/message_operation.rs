use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationReference, ControllerOperationReferenceKind, ControllerOperationStage,
    ControllerOperationStatus, CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

#[test]
fn command_uses_tags() {
    let command = ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: "op-1".into(),
        correlation_id: "corr-1".into(),
        command: ControllerOperationCommand::SendText {
            text: "hello".into(),
            target_endpoint_id: "endpoint-b".into(),
            participant_id: Some("santi".into()),
            conversation_id: None,
        },
    };

    let encoded = serde_json::to_value(&command).unwrap();

    assert_eq!(encoded["command"]["command"], "send-text");
    assert_eq!(encoded["schema_version"], 1);
}

#[test]
fn terminal_uses_stage() {
    let event = ControllerOperationEvent {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        event_id: "event-1".into(),
        operation_id: "op-1".into(),
        correlation_id: "corr-1".into(),
        causation_id: None,
        conversation_id: None,
        message_id: None,
        stage: ControllerOperationStage::OperationCompleted,
        status: ControllerOperationStatus::Completed,
        occurred_at: "2026-05-04T00:00:00Z".into(),
        detail: None,
        references: vec![],
        message_delta: None,
        snapshot: None,
    };

    assert!(event.is_terminal());
}

#[test]
fn references_remain_compatible() {
    let event = ControllerOperationEvent {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        event_id: "event-1".into(),
        operation_id: "op-1".into(),
        correlation_id: "corr-1".into(),
        causation_id: None,
        conversation_id: Some("conv-1".into()),
        message_id: Some("msg-1".into()),
        stage: ControllerOperationStage::DeliveryCompleted,
        status: ControllerOperationStatus::Completed,
        occurred_at: "2026-05-04T00:00:00Z".into(),
        detail: None,
        references: vec![ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::ProtocolEnvelope,
            ledger_id: None,
            fact_id: None,
            message_id: Some("msg-1".into()),
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: None,
            endpoint_id: Some("endpoint-a".into()),
            envelope_id: Some("env-1".into()),
            reply_id: Some("reply-1".into()),
            detail: None,
        }],
        message_delta: None,
        snapshot: None,
    };

    let encoded = serde_json::to_value(&event).unwrap();
    assert_eq!(
        encoded["references"][0]["reference_kind"],
        "protocol-envelope"
    );
    assert_eq!(encoded["references"][0]["envelope_id"], "env-1");

    let mut old_event = encoded;
    old_event.as_object_mut().unwrap().remove("references");
    let decoded: ControllerOperationEvent = serde_json::from_value(old_event).unwrap();
    assert!(decoded.references.is_empty());
}
