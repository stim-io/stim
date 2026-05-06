use stim_shared::message_operation::ControllerOperationSnapshot;

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

pub(super) fn assert_snapshot_contains_user_texts(
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

pub(super) fn assert_snapshot_has_tool_activity(
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

#[cfg(test)]
mod tests {
    use super::{
        assert_last_assistant_contains, assert_snapshot_contains_user_texts,
        assert_snapshot_message_counts,
    };

    #[test]
    fn snapshot_assertions_require_distinct_two_turn_content() {
        let snapshot = stim_shared::message_operation::ControllerOperationSnapshot {
            conversation_id: "conv-1".into(),
            message_count: 4,
            user_message_count: 2,
            assistant_message_count: 2,
            tool_activity_count: 0,
            tool_result_count: 0,
            last_user_text: Some("second".into()),
            last_assistant_text: Some("the prior text was first".into()),
            final_sent_text: Some("second".into()),
            response_text_source: Some("stim_reply_handle".into()),
            messages: vec![
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-1".into(),
                    role: "user".into(),
                    text: "first".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-2".into(),
                    role: "assistant".into(),
                    text: "assistant one".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-3".into(),
                    role: "user".into(),
                    text: "second".into(),
                },
                stim_shared::message_operation::ControllerOperationMessage {
                    id: "msg-4".into(),
                    role: "assistant".into(),
                    text: "the prior text was first".into(),
                },
            ],
            tool_activities: vec![],
        };

        assert!(assert_snapshot_contains_user_texts(
            &snapshot,
            &["first", "second"],
            "final-reload",
        )
        .is_ok());
        assert!(assert_snapshot_message_counts(&snapshot, 2, 2, "final-reload").is_ok());
        assert!(assert_last_assistant_contains(&snapshot, "first", "final-reload").is_ok());
        assert!(
            assert_snapshot_contains_user_texts(&snapshot, &["missing"], "final-reload",).is_err()
        );
        assert!(assert_last_assistant_contains(&snapshot, "missing", "final-reload").is_err());

        let mut empty_assistant_snapshot = snapshot.clone();
        for message in &mut empty_assistant_snapshot.messages {
            if message.role == "assistant" {
                message.text.clear();
            }
        }
        assert!(
            assert_snapshot_message_counts(&empty_assistant_snapshot, 2, 2, "final-reload")
                .is_err()
        );
    }
}
