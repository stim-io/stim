type RendererProbeRequest = {
  request_id: string;
  requested_at: string;
  probe: {
    probe: "landing-basics" | "messaging-state";
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
                kind: "messaging-state";
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

function sessionDrawerCollapsed(): boolean {
  return (
    document.querySelector('[data-probe="session-drawer"]')?.getAttribute(
      "data-collapsed",
    ) === "true"
  );
}

function activeSessionIdText(): string | null {
  return (
    document.querySelector<HTMLElement>('[data-probe="active-session-item"]')
      ?.dataset.sessionId ?? null
  );
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
  return chatBubbles().filter((bubble) => !role || bubble.dataset.role === role);
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
    lastAssistantRichContent()?.querySelector('[data-probe="rich-content-fragment"]'),
  );
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

function buildMessagingStateResponse(
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
          kind: "messaging-state",
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
        },
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
        : request.probe.probe === "messaging-state"
          ? buildMessagingStateResponse(request)
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
