import {
  readRendererMessagingState,
  type RendererMessagingStateSnapshot,
} from "./messagingState";

type RendererActionRequest = {
  request_id: string;
  requested_at: string;
  action: {
    action: "messaging-send";
    text: string;
    target_endpoint_id: string | null;
  };
};

type RendererActionResponse = {
  request_id: string;
  requested_at: string;
  result:
    | {
        kind: "success";
        snapshot: {
          action: "messaging-send";
          submitted_text: string;
          target_endpoint_id: string;
          before: RendererMessagingStateSnapshot;
          after: RendererMessagingStateSnapshot;
        };
      }
    | {
        kind: "failure";
        reason: "action-failed" | "action-timed-out";
        detail: string | null;
      };
};

const REQUEST_EVENT = "stim://inspection/renderer-action-request";
const RESPONSE_EVENT = "stim://inspection/renderer-action-response";

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function messageInput(): HTMLInputElement | null {
  return document.querySelector<HTMLInputElement>(
    '[data-probe="message-input"]',
  );
}

function targetEndpointInput(): HTMLInputElement | null {
  return document.querySelector<HTMLInputElement>(
    '[data-probe="target-endpoint-input"]',
  );
}

function sendButton(): HTMLButtonElement | null {
  return document.querySelector<HTMLButtonElement>(
    '[data-probe="landing-actions"] button',
  );
}

function setInputValue(input: HTMLInputElement, value: string) {
  input.value = value;
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

async function waitForEnabledSendButton(timeoutMs: number) {
  const started = Date.now();

  while (Date.now() - started < timeoutMs) {
    const button = sendButton();
    if (button && !button.disabled) {
      return button;
    }

    await sleep(50);
  }

  throw new Error("send button did not become enabled");
}

async function waitForSentMessagingState(
  before: RendererMessagingStateSnapshot,
  submittedText: string,
  timeoutMs: number,
) {
  const started = Date.now();

  while (Date.now() - started < timeoutMs) {
    const after = readRendererMessagingState();
    const visibleUserText =
      after.last_user_text?.includes(submittedText) ?? false;
    const visibleAssistantText = Boolean(after.last_assistant_text);
    const countAdvanced = after.chat_entry_count > before.chat_entry_count;
    const hasConversation = Boolean(after.active_conversation_id);
    const idleAgain = after.primary_action_label === "Send message";

    if (
      visibleUserText &&
      visibleAssistantText &&
      countAdvanced &&
      hasConversation &&
      idleAgain &&
      !after.error_message
    ) {
      return after;
    }

    if (after.error_message) {
      throw new Error(after.error_message);
    }

    await sleep(200);
  }

  throw new Error("messaging UI did not reach sent state before timeout");
}

async function performMessagingSend(action: RendererActionRequest["action"]) {
  const text = action.text.trim();
  if (!text) {
    throw new Error("messaging smoke text must not be empty");
  }

  const message = messageInput();
  if (!message) {
    throw new Error("message input not found");
  }

  const target = targetEndpointInput();
  if (!target) {
    throw new Error("target endpoint input not found");
  }

  const before = readRendererMessagingState();
  const targetEndpointId = action.target_endpoint_id?.trim() || target.value;
  if (!targetEndpointId) {
    throw new Error("target endpoint id must not be empty");
  }

  setInputValue(target, targetEndpointId);
  setInputValue(message, text);
  const button = await waitForEnabledSendButton(5_000);
  button.click();
  const after = await waitForSentMessagingState(before, text, 45_000);

  return {
    action: "messaging-send" as const,
    submitted_text: text,
    target_endpoint_id: targetEndpointId,
    before,
    after,
  };
}

function buildFailureResponse(
  request: RendererActionRequest,
  error: unknown,
): RendererActionResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "failure",
      reason: "action-failed",
      detail: error instanceof Error ? error.message : String(error),
    },
  };
}

async function handleActionRequest(request: RendererActionRequest) {
  const { emit } = await import("@tauri-apps/api/event");

  try {
    const snapshot =
      request.action.action === "messaging-send"
        ? await performMessagingSend(request.action)
        : null;

    if (!snapshot) {
      throw new Error("unsupported renderer action");
    }

    await emit(RESPONSE_EVENT, {
      request_id: request.request_id,
      requested_at: request.requested_at,
      result: {
        kind: "success",
        snapshot,
      },
    } satisfies RendererActionResponse);
  } catch (error) {
    await emit(RESPONSE_EVENT, buildFailureResponse(request, error));
  }
}

export async function setupInspectionActions() {
  if (!hasTauriRuntime()) {
    return;
  }

  const { listen } = await import("@tauri-apps/api/event");

  await listen<RendererActionRequest>(REQUEST_EVENT, async (event) => {
    if (!event.payload) {
      return;
    }

    await handleActionRequest(event.payload);
  });
}
