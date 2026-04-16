type RendererProbeRequest = {
  request_id: string;
  requested_at: string;
  probe:
    | {
        probe:
          | "landing-basics"
          | "first-message-result"
          | "multi-turn-result"
          | "context-chat-result";
      }
    | {
        probe: "chat-turn";
        text: string;
        reset: boolean;
      };
};

type RendererProbeResponse = {
  request_id: string;
  requested_at: string;
  result:
    | {
        kind: "success";
              snapshot: {
                inspected_at: string;
                probe:
            | {
                kind: "landing-basics";
                document_ready_state: string;
                document_title: string;
                landing_shell_present: boolean;
                landing_card_present: boolean;
                session_drawer_present: boolean;
                session_drawer_collapsed: boolean;
                landing_title_text: string | null;
                primary_action_label: string | null;
                active_session_id: string | null;
              }
            | {
                kind: "first-message-result";
                document_ready_state: string;
                response_text: string | null;
                response_source: string | null;
                final_sent_text: string | null;
                assistant_response_content_kind: string | null;
                assistant_fragment_present: boolean;
                error_message: string | null;
                primary_action_label: string | null;
              }
            | {
                kind: "multi-turn-result";
                document_ready_state: string;
                first_response_text: string | null;
                second_response_text: string | null;
                first_final_sent_text: string | null;
                second_final_sent_text: string | null;
                first_conversation_id: string | null;
                second_conversation_id: string | null;
                same_conversation_reused: boolean;
                chat_entry_count: number;
                user_entry_count: number;
                assistant_entry_count: number;
                assistant_response_content_kind: string | null;
                assistant_fragment_present: boolean;
                error_message: string | null;
                primary_action_label: string | null;
              }
            | {
                kind: "context-chat-result";
                document_ready_state: string;
                remember_response_text: string | null;
                recall_response_text: string | null;
                count_response_text: string | null;
                conversation_id: string | null;
                same_conversation_reused: boolean;
                recall_matches_expected_phrase: boolean;
                count_matches_expected_words: boolean;
                chat_entry_count: number;
                user_entry_count: number;
                assistant_entry_count: number;
                error_message: string | null;
                primary_action_label: string | null;
              }
            | {
                kind: "chat-turn-result";
                document_ready_state: string;
                sent_text: string;
                response_text: string | null;
                final_sent_text: string | null;
                conversation_id: string | null;
                chat_entry_count: number;
                user_entry_count: number;
                assistant_entry_count: number;
                assistant_response_content_kind: string | null;
                assistant_fragment_present: boolean;
                error_message: string | null;
                primary_action_label: string | null;
              };
        };
      }
    | {
        kind: "failure";
        reason: "probe-failed";
      };
};

const REQUEST_EVENT = "stim://inspection/renderer-probe-request";
const RESPONSE_EVENT = "stim://inspection/renderer-probe-response";

function timestampNow(): string {
  const now = Date.now();
  return `${Math.floor(now / 1000)}-${String(now % 1000).padStart(3, "0")}`;
}

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function textContentFor(selector: string): string | null {
  return document.querySelector(selector)?.textContent?.trim() || null;
}

function primaryActionLabel(): string | null {
  return textContentFor('[data-probe="landing-actions"] button');
}

function drawerToggleButton(): HTMLButtonElement | null {
  return document.querySelector<HTMLButtonElement>(
    '[data-probe="session-drawer-toggle"]',
  );
}

function newConversationButton(): HTMLButtonElement | null {
  return document.querySelector<HTMLButtonElement>(
    '[data-probe="new-conversation-button"]',
  );
}

function sessionDrawerCollapsed(): boolean {
  return (
    document.querySelector('[data-probe="session-drawer"]')?.getAttribute(
      'data-collapsed',
    ) === 'true'
  );
}

function activeSessionIdText(): string | null {
  return document
    .querySelector<HTMLElement>('[data-probe="active-session-item"]')
    ?.dataset.sessionId ?? null;
}

function primaryActionReady(): boolean {
  return primaryActionLabel() === "Send message";
}

function inputFor(selector: string): HTMLInputElement | null {
  return document.querySelector<HTMLInputElement>(selector);
}

function actionButton(index: number): HTMLButtonElement | null {
  return (
    document.querySelectorAll<HTMLButtonElement>(
      '[data-probe="landing-actions"] button',
    )[index] ?? null
  );
}

function setInputValue(input: HTMLInputElement | null, value: string) {
  if (!input) {
    return;
  }

  input.focus();
  input.value = value;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
}

function clickButton(button: HTMLButtonElement | null) {
  button?.click();
}

function conversationIdText(): string | null {
  return textContentFor('[data-probe="active-conversation-id"]');
}

function chatBubbles() {
  return Array.from(
    document.querySelectorAll<HTMLElement>(
      '[data-probe="chat-thread"] .chat-bubble',
    ),
  );
}

function chatEntryCountFor(role?: "user" | "assistant"): number {
  return chatBubbles().filter((bubble) => !role || bubble.dataset.role === role)
    .length;
}

function lastAssistantRichContent(): HTMLElement | null {
  const lastAssistantBubble = chatBubbles()
    .filter((bubble) => bubble.dataset.role === "assistant")
    .at(-1);

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
    lastAssistantRichContent()?.querySelector('[data-probe="rich-content-fragment"]'),
  );
}

async function waitFor(condition: () => boolean, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (condition()) {
      return true;
    }

    await new Promise((resolve) => window.setTimeout(resolve, 200));
  }

  return false;
}

function normalizeText(value: string | null): string {
  return (value ?? "").trim().toLowerCase();
}

function firstMessageSnapshot() {
  return {
    kind: "first-message-result" as const,
    document_ready_state: document.readyState,
    response_text: textContentFor('[data-probe="last-response-text"]'),
    response_source: textContentFor('[data-probe="last-response-source"]'),
    final_sent_text: textContentFor('[data-probe="last-final-sent-text"]'),
    assistant_response_content_kind: lastAssistantResponseContentKind(),
    assistant_fragment_present: lastAssistantFragmentPresent(),
    error_message: textContentFor('[data-probe="last-error-message"]'),
    primary_action_label: primaryActionLabel(),
  };
}

function buildLandingBasicsResponse(
  request: RendererProbeRequest,
): RendererProbeResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: {
          kind: "landing-basics",
          document_ready_state: document.readyState,
          document_title: document.title,
          landing_shell_present: Boolean(
            document.querySelector('[data-probe="landing-shell"]'),
          ),
          landing_card_present: Boolean(
            document.querySelector('[data-probe="landing-card"]'),
          ),
          session_drawer_present: Boolean(
            document.querySelector('[data-probe="session-drawer"]'),
          ),
          session_drawer_collapsed: sessionDrawerCollapsed(),
          landing_title_text: textContentFor('[data-probe="landing-title"]'),
          primary_action_label: primaryActionLabel(),
          active_session_id: activeSessionIdText(),
        },
      },
    },
  };
}

function buildFirstMessageResultResponse(
  request: RendererProbeRequest,
): RendererProbeResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: firstMessageSnapshot(),
      },
    },
  };
}

async function collectFirstMessageResultResponse(
  request: RendererProbeRequest,
): Promise<RendererProbeResponse> {
  const existing = firstMessageSnapshot();

  if (!existing.response_text) {
    const button = actionButton(0);

    clickButton(button);

    if (
      await waitFor(() => {
        const snapshot = firstMessageSnapshot();
        return Boolean(snapshot.response_text || snapshot.error_message);
      }, 20000)
    ) {
      const snapshot = firstMessageSnapshot();
      return {
        request_id: request.request_id,
        requested_at: request.requested_at,
        result: {
          kind: "success",
          snapshot: {
            inspected_at: timestampNow(),
            probe: snapshot,
          },
        },
      };
    }
  }

  return buildFirstMessageResultResponse(request);
}

function buildMultiTurnSnapshot(result: {
  firstResponseText: string | null;
  secondResponseText: string | null;
  firstFinalSentText: string | null;
  secondFinalSentText: string | null;
  firstConversationId: string | null;
  secondConversationId: string | null;
}) {
  return {
    kind: "multi-turn-result" as const,
    document_ready_state: document.readyState,
    first_response_text: result.firstResponseText,
    second_response_text: result.secondResponseText,
    first_final_sent_text: result.firstFinalSentText,
    second_final_sent_text: result.secondFinalSentText,
    first_conversation_id: result.firstConversationId,
    second_conversation_id: result.secondConversationId,
    same_conversation_reused: Boolean(
      result.firstConversationId &&
      result.secondConversationId &&
      result.firstConversationId === result.secondConversationId,
    ),
    chat_entry_count: chatEntryCountFor(),
    user_entry_count: chatEntryCountFor("user"),
    assistant_entry_count: chatEntryCountFor("assistant"),
    assistant_response_content_kind: lastAssistantResponseContentKind(),
    assistant_fragment_present: lastAssistantFragmentPresent(),
    error_message: textContentFor('[data-probe="last-error-message"]'),
    primary_action_label: primaryActionLabel(),
  };
}

async function collectMultiTurnResultResponse(
  request: RendererProbeRequest,
): Promise<RendererProbeResponse> {
  const messageInput = inputFor('[data-probe="message-input"]');
  const sendButton = actionButton(0);
  const resetButton = newConversationButton();

  clickButton(resetButton);
  await waitFor(
    () =>
      chatEntryCountFor() === 0 &&
      !textContentFor('[data-probe="last-error-message"]'),
    2000,
  );

  setInputValue(messageInput, "hello from stim ui");
  const firstCountBefore = chatEntryCountFor();
  clickButton(sendButton);

  const firstTurnReady = await waitFor(() => {
    const snapshot = firstMessageSnapshot();
    return (
      Boolean(snapshot.error_message) ||
      chatEntryCountFor() >= firstCountBefore + 2
    );
  }, 20000);

  const firstSnapshot = firstMessageSnapshot();
  const firstConversationId = conversationIdText();

  if (!firstTurnReady || firstSnapshot.error_message || !firstConversationId) {
    return {
      request_id: request.request_id,
      requested_at: request.requested_at,
      result: {
        kind: "success",
        snapshot: {
          inspected_at: timestampNow(),
          probe: buildMultiTurnSnapshot({
            firstResponseText: firstSnapshot.response_text,
            secondResponseText: null,
            firstFinalSentText: firstSnapshot.final_sent_text,
            secondFinalSentText: null,
            firstConversationId,
            secondConversationId: conversationIdText(),
          }),
        },
      },
    };
  }

  setInputValue(messageInput, "hello again from stim ui");
  const secondCountBefore = chatEntryCountFor();
  clickButton(sendButton);

  await waitFor(() => {
    const snapshot = firstMessageSnapshot();
    return (
      Boolean(snapshot.error_message) ||
      chatEntryCountFor() >= secondCountBefore + 2
    );
  }, 20000);

  const secondSnapshot = firstMessageSnapshot();
  const secondConversationId = conversationIdText();

  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: buildMultiTurnSnapshot({
          firstResponseText: firstSnapshot.response_text,
          secondResponseText: secondSnapshot.response_text,
          firstFinalSentText: firstSnapshot.final_sent_text,
          secondFinalSentText: secondSnapshot.final_sent_text,
          firstConversationId,
          secondConversationId,
        }),
      },
    },
  };
}

type ScriptedTurnResult = {
  responseText: string | null;
  finalSentText: string | null;
  conversationId: string | null;
  errorMessage: string | null;
};

async function sendScriptedTurn(
  text: string,
  messageInput: HTMLInputElement | null,
  sendButton: HTMLButtonElement | null,
): Promise<ScriptedTurnResult> {
  setInputValue(messageInput, text);
  const countBefore = chatEntryCountFor();
  clickButton(sendButton);

  await waitFor(() => {
    const snapshot = firstMessageSnapshot();
    return (
      Boolean(snapshot.error_message) ||
      (chatEntryCountFor() >= countBefore + 2 && primaryActionReady())
    );
  }, 20000);

  const snapshot = firstMessageSnapshot();
  return {
    responseText: snapshot.response_text,
    finalSentText: snapshot.final_sent_text,
    conversationId: conversationIdText(),
    errorMessage: snapshot.error_message,
  };
}

function buildChatTurnSnapshot(sentText: string) {
  const snapshot = firstMessageSnapshot();
  return {
    kind: "chat-turn-result" as const,
    document_ready_state: document.readyState,
    sent_text: sentText,
    response_text: snapshot.response_text,
    final_sent_text: snapshot.final_sent_text,
    conversation_id: conversationIdText(),
    chat_entry_count: chatEntryCountFor(),
    user_entry_count: chatEntryCountFor("user"),
    assistant_entry_count: chatEntryCountFor("assistant"),
    assistant_response_content_kind: snapshot.assistant_response_content_kind,
    assistant_fragment_present: snapshot.assistant_fragment_present,
    error_message: snapshot.error_message,
    primary_action_label: primaryActionLabel(),
  };
}

async function collectChatTurnResponse(
  request: {
    request_id: string;
    requested_at: string;
    probe: { probe: "chat-turn"; text: string; reset: boolean };
  },
): Promise<RendererProbeResponse> {
  const messageInput = inputFor('[data-probe="message-input"]');
  const sendButton = actionButton(0);
  const resetButton = newConversationButton();

  if (request.probe.reset) {
    clickButton(resetButton);
    await waitFor(
      () =>
        chatEntryCountFor() === 0 &&
        !textContentFor('[data-probe="last-error-message"]'),
      2000,
    );
  }

  await sendScriptedTurn(request.probe.text, messageInput, sendButton);

  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: buildChatTurnSnapshot(request.probe.text),
      },
    },
  };
}

function buildContextChatSnapshot(result: {
  rememberResponseText: string | null;
  recallResponseText: string | null;
  countResponseText: string | null;
  conversationId: string | null;
  sameConversationReused: boolean;
}) {
  const expectedPhrase = "blue cactus";
  const normalizedRecall = normalizeText(result.recallResponseText);
  const normalizedCount = normalizeText(result.countResponseText);

  return {
    kind: "context-chat-result" as const,
    document_ready_state: document.readyState,
    remember_response_text: result.rememberResponseText,
    recall_response_text: result.recallResponseText,
    count_response_text: result.countResponseText,
    conversation_id: result.conversationId,
    same_conversation_reused: result.sameConversationReused,
    recall_matches_expected_phrase:
      normalizedRecall === expectedPhrase ||
      normalizedRecall === `"${expectedPhrase}"` ||
      normalizedRecall.includes(expectedPhrase),
    count_matches_expected_words:
      normalizedCount === "2" ||
      normalizedCount === "two" ||
      normalizedCount.startsWith("2 ") ||
      normalizedCount.startsWith("two "),
    chat_entry_count: chatEntryCountFor(),
    user_entry_count: chatEntryCountFor("user"),
    assistant_entry_count: chatEntryCountFor("assistant"),
    error_message: textContentFor('[data-probe="last-error-message"]'),
    primary_action_label: primaryActionLabel(),
  };
}

async function collectContextChatResultResponse(
  request: RendererProbeRequest,
): Promise<RendererProbeResponse> {
  const messageInput = inputFor('[data-probe="message-input"]');
  const sendButton = actionButton(0);
  const resetButton = newConversationButton();

  clickButton(resetButton);
  await waitFor(
    () =>
      chatEntryCountFor() === 0 &&
      !textContentFor('[data-probe="last-error-message"]'),
    2000,
  );

  const rememberTurn = await sendScriptedTurn(
    "For this chat, remember the phrase blue cactus. Reply only: remembered.",
    messageInput,
    sendButton,
  );

  const recallTurn = rememberTurn.errorMessage
    ? {
        responseText: null,
        finalSentText: null,
        conversationId: rememberTurn.conversationId,
        errorMessage: rememberTurn.errorMessage,
      }
    : await sendScriptedTurn(
        "What phrase did I ask you to remember? Reply only with the phrase.",
        messageInput,
        sendButton,
      );

  const countTurn = recallTurn.errorMessage
    ? {
        responseText: null,
        finalSentText: null,
        conversationId: recallTurn.conversationId,
        errorMessage: recallTurn.errorMessage,
      }
    : await sendScriptedTurn(
        "How many words are in that phrase? Reply only with the number.",
        messageInput,
        sendButton,
      );

  const conversationIds = [
    rememberTurn.conversationId,
    recallTurn.conversationId,
    countTurn.conversationId,
  ].filter((value): value is string => Boolean(value));

  const sameConversationReused =
    conversationIds.length === 3 && new Set(conversationIds).size === 1;

  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: buildContextChatSnapshot({
          rememberResponseText: rememberTurn.responseText,
          recallResponseText: recallTurn.responseText,
          countResponseText: countTurn.responseText,
          conversationId:
            countTurn.conversationId ??
            recallTurn.conversationId ??
            rememberTurn.conversationId,
          sameConversationReused,
        }),
      },
    },
  };
}

function buildFailureResponse(
  request: RendererProbeRequest,
): RendererProbeResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "failure",
      reason: "probe-failed",
    },
  };
}

async function handleProbeRequest(request: RendererProbeRequest) {
  const { emit } = await import("@tauri-apps/api/event");

  try {
    const response =
      request.probe.probe === "landing-basics"
        ? buildLandingBasicsResponse(request)
        : request.probe.probe === "first-message-result"
          ? await collectFirstMessageResultResponse(request)
          : request.probe.probe === "multi-turn-result"
            ? await collectMultiTurnResultResponse(request)
            : request.probe.probe === "context-chat-result"
              ? await collectContextChatResultResponse(request)
              : request.probe.probe === "chat-turn"
                ? await collectChatTurnResponse({
                    request_id: request.request_id,
                    requested_at: request.requested_at,
                    probe: request.probe,
                  })
                : buildFailureResponse(request);

    await emit(RESPONSE_EVENT, response);
  } catch {
    await emit(RESPONSE_EVENT, buildFailureResponse(request));
  }
}

export async function setupInspectionProbes() {
  if (!hasTauriRuntime()) {
    return;
  }

  const { listen } = await import("@tauri-apps/api/event");

  await listen<RendererProbeRequest>(REQUEST_EVENT, async (event) => {
    if (!event.payload) {
      return;
    }

    await handleProbeRequest(event.payload);
  });
}
