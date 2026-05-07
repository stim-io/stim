import { fetchControllerRuntimeSnapshot } from "./runtime";

export type LifecycleTraceStep = {
  operation: string;
  sent_envelope_id: string;
  ack_envelope_id: string;
  ack_message_id: string;
  ack_version: number;
  response_text: string;
  response_text_source: string;
};

export type MessageLayoutHint = {
  layout_family: string | null;
  min_height_px: number | null;
  max_height_px: number | null;
  vertical_pressure: string | null;
};

export type MessagePart =
  | {
      kind: "text";
      text: string;
    }
  | {
      kind: "stim_dom_fragment";
      tree: unknown;
    }
  | {
      kind: "raw_html";
      html: string;
    };

export type MessageContent = {
  parts: MessagePart[];
  layout_hint: MessageLayoutHint | null;
};

export type LifecycleProof = {
  create_ack_version: number;
  patch_ack_version: number;
  fix_ack_version: number;
  final_message_version: number;
  expected_final_text: string;
  controller_final_text: string;
  final_text_matches_expected: boolean;
  version_progression_valid: boolean;
};

export type FirstMessageResponse = {
  conversation_id: string;
  message_id: string;
  target_endpoint_id: string;
  participant_id: string | null;
  sent_text: string;
  final_sent_text: string;
  final_sent_content: MessageContent;
  final_message_version: number;
  response_text: string;
  response_content: MessageContent;
  response_text_source: string;
  sent_envelope_id: string;
  response_envelope_id: string;
  receipt_result: string;
  receipt_detail: string | null;
  lifecycle_trace: LifecycleTraceStep[];
  lifecycle_proof: LifecycleProof;
};

export type TranscriptMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  author: string;
  sent_at_label: string;
  content: MessageContent;
  delivery_state: "sent" | "sending" | "failed" | null;
  meta_label: string | null;
};

export type ConversationToolActivity = {
  tool_call_id: string;
  tool_name: string;
  tool_call_seq: number;
  result_state: string;
  tool_result_id: string | null;
  tool_result_seq: number | null;
  exit_code: number | null;
  duration_ms: number | null;
  stdout_chars: number | null;
  stderr_chars: number | null;
  output_summary: string | null;
};

export type ConversationTranscriptResponse = {
  conversation_id: string;
  messages: TranscriptMessage[];
  tool_activities: ConversationToolActivity[];
};

export type ControllerOperationStage =
  | "command-accepted"
  | "delivery-target-resolved"
  | "delivery-started"
  | "message-chunk-appended"
  | "conversation-selected"
  | "delivery-completed"
  | "transcript-loaded"
  | "operation-completed"
  | "operation-failed";

export type ControllerOperationStatus =
  | "accepted"
  | "running"
  | "completed"
  | "failed";

export type ControllerOperationMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  text: string;
};

export type ControllerOperationMessageDelta = {
  message_id: string;
  role: "user" | "assistant" | "system";
  text: string;
};

export type ControllerOperationSnapshot = {
  conversation_id: string;
  message_count: number;
  user_message_count: number;
  assistant_message_count: number;
  tool_activity_count: number;
  tool_result_count: number;
  last_user_text: string | null;
  last_assistant_text: string | null;
  final_sent_text: string | null;
  response_text_source: string | null;
  messages: ControllerOperationMessage[];
  tool_activities: ConversationToolActivity[];
};

export type ControllerOperationReference = {
  reference_kind: string;
  ledger_id: string | null;
  fact_id: string | null;
  message_id: string | null;
  content_id: string | null;
  revision_id: string | null;
  relation_id: string | null;
  participant_id: string | null;
  endpoint_id: string | null;
  envelope_id: string | null;
  reply_id: string | null;
  detail: string | null;
};

export type ControllerOperationEvent = {
  schema_version: number;
  event_id: string;
  operation_id: string;
  correlation_id: string;
  causation_id: string | null;
  conversation_id: string | null;
  message_id: string | null;
  stage: ControllerOperationStage;
  status: ControllerOperationStatus;
  occurred_at: string;
  detail: string | null;
  references: ControllerOperationReference[];
  message_delta: ControllerOperationMessageDelta | null;
  snapshot: ControllerOperationSnapshot | null;
};

export async function sendFirstMessage(
  text: string,
  targetEndpointId: string,
  participantId?: string | null,
  conversationId?: string | null,
) {
  const snapshot = await fetchControllerRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Controller HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/messages/roundtrip`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        text,
        target_endpoint_id: targetEndpointId,
        participant_id: participantId ?? null,
        conversation_id: conversationId ?? null,
      }),
    },
  );

  if (!response.ok) {
    throw new Error(`Controller roundtrip failed: ${response.status}`);
  }

  return response.json() as Promise<FirstMessageResponse>;
}

export async function sendTextOperation(
  text: string,
  targetEndpointId: string,
  participantId: string | null | undefined,
  conversationId: string | null | undefined,
  onEvent: (event: ControllerOperationEvent) => void,
) {
  const snapshot = await fetchControllerRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Controller HTTP base URL unavailable");
  }

  const wsUrl = controllerOperationWsUrl(snapshot.http_base_url);
  const operationId = createClientOperationId("op");
  const command = {
    schema_version: 1,
    operation_id: operationId,
    correlation_id: createClientOperationId("corr"),
    command: {
      command: "send-text",
      text,
      target_endpoint_id: targetEndpointId,
      participant_id: participantId ?? null,
      conversation_id: conversationId ?? null,
    },
  };

  return new Promise<ControllerOperationEvent>((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timeout = window.setTimeout(() => {
      socket.close();
      reject(new Error("Controller operation timed out"));
    }, 120_000);

    socket.addEventListener("open", () => {
      socket.send(JSON.stringify(command));
    });

    socket.addEventListener("message", (message) => {
      try {
        const event = JSON.parse(String(message.data)) as ControllerOperationEvent;
        onEvent(event);

        if (isTerminalOperationEvent(event)) {
          window.clearTimeout(timeout);
          socket.close();
          if (
            event.stage === "operation-failed" ||
            event.status === "failed"
          ) {
            reject(new Error(event.detail ?? "Controller operation failed"));
          } else {
            resolve(event);
          }
        }
      } catch (error) {
        window.clearTimeout(timeout);
        socket.close();
        reject(error);
      }
    });

    socket.addEventListener("error", () => {
      window.clearTimeout(timeout);
      reject(new Error("Controller operation WebSocket failed"));
    });

    socket.addEventListener("close", () => {
      window.clearTimeout(timeout);
    });
  });
}

export async function fetchConversationTranscript(conversationId: string) {
  const snapshot = await fetchControllerRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Controller HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/conversations/${encodeURIComponent(
      conversationId,
    )}/messages`,
  );

  if (!response.ok) {
    throw new Error(`Controller transcript fetch failed: ${response.status}`);
  }

  return response.json() as Promise<ConversationTranscriptResponse>;
}

function controllerOperationWsUrl(controllerBaseUrl: string) {
  const baseUrl = controllerBaseUrl.replace(/\/+$/, "");
  if (baseUrl.startsWith("http://")) {
    return `${baseUrl.replace(/^http:\/\//, "ws://")}/api/v1/controller/operations/ws`;
  }
  if (baseUrl.startsWith("https://")) {
    return `${baseUrl.replace(/^https:\/\//, "wss://")}/api/v1/controller/operations/ws`;
  }

  throw new Error(`Unsupported controller URL: ${controllerBaseUrl}`);
}

function isTerminalOperationEvent(event: ControllerOperationEvent) {
  return event.stage === "operation-completed" || event.stage === "operation-failed";
}

function createClientOperationId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
