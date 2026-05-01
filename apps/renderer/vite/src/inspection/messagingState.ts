export type RendererMessagingStateSnapshot = {
  document_ready_state: string;
  active_session_id: string | null;
  active_conversation_id: string | null;
  chat_entry_count: number;
  user_entry_count: number;
  assistant_entry_count: number;
  last_user_text: string | null;
  last_assistant_text: string | null;
  response_text: string | null;
  response_source: string | null;
  final_sent_text: string | null;
  assistant_response_content_kind: string | null;
  assistant_fragment_present: boolean;
  error_message: string | null;
  primary_action_label: string | null;
};

export function textContentFor(selector: string): string | null {
  return document.querySelector(selector)?.textContent?.trim() || null;
}

export function primaryActionLabel(): string | null {
  return textContentFor('[data-probe="landing-actions"] button');
}

export function activeSessionIdText(): string | null {
  return (
    document.querySelector<HTMLElement>('[data-probe="active-session-item"]')
      ?.dataset.sessionId ?? null
  );
}

export function readRendererMessagingState(): RendererMessagingStateSnapshot {
  return {
    document_ready_state: document.readyState,
    active_session_id: activeSessionIdText(),
    active_conversation_id: conversationIdText(),
    chat_entry_count: chatEntryCountFor(),
    user_entry_count: chatEntryCountFor("user"),
    assistant_entry_count: chatEntryCountFor("assistant"),
    last_user_text: lastBubbleText("user"),
    last_assistant_text: lastBubbleText("assistant"),
    response_text: textContentFor('[data-probe="last-response-text"]'),
    response_source: textContentFor('[data-probe="last-response-source"]'),
    final_sent_text: textContentFor('[data-probe="last-final-sent-text"]'),
    assistant_response_content_kind: lastAssistantResponseContentKind(),
    assistant_fragment_present: lastAssistantFragmentPresent(),
    error_message: textContentFor('[data-probe="last-error-message"]'),
    primary_action_label: primaryActionLabel(),
  };
}

function conversationIdText(): string | null {
  return textContentFor('[data-probe="active-conversation-id"]');
}

function chatBubbles() {
  return Array.from(
    document.querySelectorAll<HTMLElement>(
      '[data-probe="chat-thread"] [data-probe="chat-bubble"]',
    ),
  );
}

function chatBubblesFor(role?: "user" | "assistant") {
  return chatBubbles().filter(
    (bubble) => !role || bubble.dataset.role === role,
  );
}

function chatEntryCountFor(role?: "user" | "assistant"): number {
  return chatBubblesFor(role).length;
}

function lastBubbleText(role: "user" | "assistant"): string | null {
  return chatBubblesFor(role).at(-1)?.textContent?.trim() || null;
}

function lastAssistantRichContent(): HTMLElement | null {
  const lastAssistantBubble = chatBubblesFor("assistant").at(-1);

  return (
    lastAssistantBubble?.querySelector<HTMLElement>(
      '[data-probe="rich-content"]',
    ) ?? null
  );
}

function lastAssistantResponseContentKind(): string | null {
  return lastAssistantRichContent()?.dataset.contentKind ?? null;
}

function lastAssistantFragmentPresent(): boolean {
  return Boolean(
    lastAssistantRichContent()?.querySelector(
      '[data-probe="rich-content-fragment"]',
    ),
  );
}
