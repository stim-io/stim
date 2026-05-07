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

pub(crate) fn assert_snapshot_user_texts(
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

pub(crate) fn assert_snapshot_message_counts(
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

pub(crate) fn assert_last_assistant_contains(
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
