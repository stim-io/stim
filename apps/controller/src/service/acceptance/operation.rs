use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::json;
use stim_shared::message_operation::{
    ControllerOperationCommand, ControllerOperationCommandEnvelope, ControllerOperationEvent,
    ControllerOperationSnapshot, ControllerOperationStage, ControllerOperationStatus,
    CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
};

use crate::{
    model::{timestamp_now, ControllerHttpState},
    service::{run_load_transcript_operation, run_send_text_operation, OperationEventEmitter},
};

static ACCEPTANCE_COMMAND_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, serde::Serialize)]
pub(super) struct OperationRun {
    pub(super) events: Vec<ControllerOperationEvent>,
    pub(super) snapshot: ControllerOperationSnapshot,
}

#[derive(Default)]
struct VecOperationEventEmitter {
    events: Vec<ControllerOperationEvent>,
}

impl OperationEventEmitter for VecOperationEventEmitter {
    fn emit<'a>(
        &'a mut self,
        event: ControllerOperationEvent,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let event_id = event.event_id.clone();
            self.events.push(event);
            Ok(event_id)
        })
    }
}

pub(super) async fn execute_send_text(
    state: ControllerHttpState,
    text: String,
    target_endpoint_id: String,
    participant_id: Option<String>,
    conversation_id: Option<String>,
    label: &str,
) -> Result<OperationRun, String> {
    let command = command_envelope(ControllerOperationCommand::SendText {
        text: text.clone(),
        target_endpoint_id: target_endpoint_id.clone(),
        participant_id: participant_id.clone(),
        conversation_id: conversation_id.clone(),
    });
    let mut emitter = VecOperationEventEmitter::default();
    run_send_text_operation(
        state,
        &command,
        text,
        target_endpoint_id,
        participant_id,
        conversation_id,
        &mut emitter,
    )
    .await?;
    let snapshot = require_completed_snapshot(&emitter.events, label)?;
    Ok(OperationRun {
        events: emitter.events,
        snapshot,
    })
}

pub(super) async fn execute_load_transcript(
    state: ControllerHttpState,
    conversation_id: String,
    label: &str,
) -> Result<OperationRun, String> {
    let command = command_envelope(ControllerOperationCommand::LoadTranscript {
        conversation_id: conversation_id.clone(),
    });
    let mut emitter = VecOperationEventEmitter::default();
    run_load_transcript_operation(state, &command, conversation_id, &mut emitter).await?;
    let snapshot = require_completed_snapshot(&emitter.events, label)?;
    Ok(OperationRun {
        events: emitter.events,
        snapshot,
    })
}

fn command_envelope(command: ControllerOperationCommand) -> ControllerOperationCommandEnvelope {
    let sequence = ACCEPTANCE_COMMAND_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = timestamp_now();

    ControllerOperationCommandEnvelope {
        schema_version: CONTROLLER_MESSAGE_OPERATION_SCHEMA_VERSION,
        operation_id: format!("accept-op-{sequence}-{timestamp}"),
        correlation_id: format!("accept-corr-{sequence}-{timestamp}"),
        command,
    }
}

pub(super) async fn seed_participant_projection(
    stim_server_base_url: String,
    santi_base_url: String,
    participant_id: String,
    delivery_endpoint_id: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| format!("failed to build stim-server acceptance client: {error}"))?;
        let instance_id = format!("acceptance-{participant_id}");
        let body = json!({
            "agent_id": participant_id.clone(),
            "instance_id": instance_id,
            "participant_id": participant_id.clone(),
            "delivery_endpoint_id": delivery_endpoint_id.clone(),
            "label": "Santi acceptance participant",
            "agent_kind": "santi",
            "endpoint": santi_base_url,
            "profile": "acceptance",
            "capabilities": ["santi", "acceptance"],
            "status": "ready",
            "detail": "seeded by controller accept.participant-routing"
        });
        let response = client
            .put(format!(
                "{}/api/v1/agents/instances/acceptance-{}",
                stim_server_base_url.trim_end_matches('/'),
                participant_id
            ))
            .json(&body)
            .send()
            .map_err(|error| format!("failed to seed participant projection: {error}"))?;
        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .unwrap_or_else(|error| format!("<failed to read body: {error}>"));
            return Err(format!(
                "failed to seed participant projection: stim-server returned {status}: {body}"
            ));
        }

        Ok(())
    })
    .await
    .map_err(|error| format!("participant projection seed join failed: {error}"))?
}

fn require_completed_snapshot(
    events: &[ControllerOperationEvent],
    label: &str,
) -> Result<ControllerOperationSnapshot, String> {
    let terminal = events
        .last()
        .ok_or_else(|| format!("{label} produced no controller operation events"))?;

    if terminal.stage == ControllerOperationStage::OperationFailed
        || terminal.status == ControllerOperationStatus::Failed
    {
        return Err(format!(
            "{label} failed at controller stage: {}",
            terminal
                .detail
                .clone()
                .unwrap_or_else(|| "no detail".into())
        ));
    }

    if terminal.stage != ControllerOperationStage::OperationCompleted
        || terminal.status != ControllerOperationStatus::Completed
    {
        return Err(format!(
            "{label} ended without operation-completed event: {:?} {:?}",
            terminal.stage, terminal.status
        ));
    }

    terminal
        .snapshot
        .clone()
        .ok_or_else(|| format!("{label} completed without controller snapshot"))
}

pub(super) fn assert_snapshot_conversation(
    snapshot: &ControllerOperationSnapshot,
    conversation_id: &str,
    label: &str,
) -> Result<(), String> {
    if snapshot.conversation_id == conversation_id {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot conversation mismatch: expected {conversation_id}, got {}",
        snapshot.conversation_id
    ))
}

pub(super) fn assert_snapshot_user_texts(
    snapshot: &ControllerOperationSnapshot,
    texts: &[&str],
    label: &str,
) -> Result<(), String> {
    for text in texts {
        if !snapshot
            .messages
            .iter()
            .any(|message| message.role == "user" && message.text == *text)
        {
            return Err(format!(
                "{label} snapshot did not contain submitted user text '{text}' in conversation {}",
                snapshot.conversation_id
            ));
        }
    }

    Ok(())
}

pub(super) fn assert_snapshot_message_counts(
    snapshot: &ControllerOperationSnapshot,
    min_user_messages: usize,
    min_assistant_messages: usize,
    label: &str,
) -> Result<(), String> {
    let user_message_count = snapshot
        .messages
        .iter()
        .filter(|message| message.role == "user" && !message.text.trim().is_empty())
        .count();
    let assistant_message_count = snapshot
        .messages
        .iter()
        .filter(|message| message.role == "assistant" && !message.text.trim().is_empty())
        .count();

    if user_message_count >= min_user_messages && assistant_message_count >= min_assistant_messages
    {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot had insufficient messages in conversation {}: users={}, assistants={}, expected at least users={}, assistants={}",
        snapshot.conversation_id,
        user_message_count,
        assistant_message_count,
        min_user_messages,
        min_assistant_messages,
    ))
}

pub(super) fn assert_snapshot_tool_activity(
    snapshot: &ControllerOperationSnapshot,
    label: &str,
) -> Result<(), String> {
    if snapshot.tool_activity_count == 0 || snapshot.tool_activities.is_empty() {
        return Err(format!(
            "{label} snapshot did not expose tool activity in conversation {}",
            snapshot.conversation_id
        ));
    }

    if snapshot
        .tool_activities
        .iter()
        .any(|activity| activity.tool_result_id.is_some() && activity.result_state == "completed")
    {
        return Ok(());
    }

    Err(format!(
        "{label} snapshot exposed tool calls but no completed tool result in conversation {}",
        snapshot.conversation_id
    ))
}

pub(super) fn assert_last_assistant_contains(
    snapshot: &ControllerOperationSnapshot,
    expected_text: &str,
    label: &str,
) -> Result<(), String> {
    let Some(last_assistant_text) = snapshot.last_assistant_text.as_deref() else {
        return Err(format!(
            "{label} snapshot did not contain a last assistant message in conversation {}",
            snapshot.conversation_id
        ));
    };

    if last_assistant_text.contains(expected_text) {
        return Ok(());
    }

    Err(format!(
        "{label} last assistant text did not include expected prior user text '{expected_text}' in conversation {}; actual last assistant text: {last_assistant_text:?}",
        snapshot.conversation_id
    ))
}

pub(super) fn assert_delivery_target_event(
    events: &[ControllerOperationEvent],
    participant_id: &str,
    delivery_endpoint_id: &str,
) -> Result<(), String> {
    let event = events
        .iter()
        .find(|event| event.stage == ControllerOperationStage::DeliveryTargetResolved)
        .ok_or_else(|| "participant-send did not emit delivery-target-resolved".to_string())?;
    let detail = event.detail.as_deref().unwrap_or_default();
    let expected =
        format!("resolved participant {participant_id} to endpoint {delivery_endpoint_id}");

    if detail != expected {
        return Err(format!(
            "participant-send resolved unexpected delivery target: {detail}"
        ));
    }

    Ok(())
}
