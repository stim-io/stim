use stim_shared::inspection::RendererMessagingStateSnapshot;

pub(super) fn assert_renderer_message_state(
    snapshot: &RendererMessagingStateSnapshot,
    expected_last_user_text: &str,
    expected_assistant_text_fragment: Option<&str>,
    min_user_entries: usize,
    min_assistant_entries: usize,
    label: &str,
) -> Result<(), String> {
    if let Some(error) = snapshot.error_message.as_deref() {
        return Err(format!("renderer {label} reported visible error: {error}"));
    }
    if snapshot.active_conversation_id.is_none() {
        return Err(format!(
            "renderer {label} did not expose an active conversation"
        ));
    }
    if snapshot.user_entry_count < min_user_entries
        || snapshot.assistant_entry_count < min_assistant_entries
    {
        return Err(format!(
            "renderer {label} had insufficient visible messages: users={}, assistants={}, expected at least users={}, assistants={}",
            snapshot.user_entry_count,
            snapshot.assistant_entry_count,
            min_user_entries,
            min_assistant_entries,
        ));
    }
    if !snapshot
        .last_user_text
        .as_deref()
        .is_some_and(|text| text.contains(expected_last_user_text))
    {
        return Err(format!(
            "renderer {label} last user text did not include expected text '{expected_last_user_text}'"
        ));
    }
    if let Some(expected_assistant_text_fragment) = expected_assistant_text_fragment {
        if !snapshot
            .last_assistant_text
            .as_deref()
            .is_some_and(|text| text.contains(expected_assistant_text_fragment))
        {
            return Err(format!(
                "renderer {label} last assistant text did not include expected text '{expected_assistant_text_fragment}'"
            ));
        }
    } else if snapshot
        .last_assistant_text
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err(format!(
            "renderer {label} did not expose a visible assistant reply"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use stim_shared::inspection::RendererMessagingStateSnapshot;

    use super::assert_renderer_message_state;

    #[test]
    fn renderer_continuation_state_requires_visible_marker_reply() {
        let snapshot = RendererMessagingStateSnapshot {
            document_ready_state: "complete".into(),
            active_session_id: Some("live-controller".into()),
            active_conversation_id: Some("conv-1".into()),
            chat_entry_count: 4,
            user_entry_count: 2,
            assistant_entry_count: 2,
            last_user_text: Some(super::super::renderer::CONTINUATION_FOLLOWUP_TEXT.into()),
            last_assistant_text: Some("marker kiwi lantern".into()),
            response_text: Some("marker kiwi lantern".into()),
            response_source: Some("stim_reply_handle".into()),
            final_sent_text: Some(super::super::renderer::CONTINUATION_FOLLOWUP_TEXT.into()),
            tool_activity_count: 0,
            latest_tool_activity_summary: None,
            assistant_response_content_kind: Some("text".into()),
            assistant_fragment_present: false,
            error_message: None,
            primary_action_label: Some("Send message".into()),
        };

        assert!(assert_renderer_message_state(
            &snapshot,
            super::super::renderer::CONTINUATION_FOLLOWUP_TEXT,
            Some("marker kiwi lantern"),
            2,
            2,
            "second-turn",
        )
        .is_ok());
        assert!(assert_renderer_message_state(
            &snapshot,
            super::super::renderer::CONTINUATION_FOLLOWUP_TEXT,
            Some("missing"),
            2,
            2,
            "second-turn",
        )
        .is_err());
    }
}
