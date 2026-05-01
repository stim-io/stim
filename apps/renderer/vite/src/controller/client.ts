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

export type ConversationTranscriptResponse = {
  conversation_id: string;
  messages: TranscriptMessage[];
};

export async function sendFirstMessage(
  text: string,
  targetEndpointId: string,
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
        conversation_id: conversationId ?? null,
      }),
    },
  );

  if (!response.ok) {
    throw new Error(`Controller roundtrip failed: ${response.status}`);
  }

  return response.json() as Promise<FirstMessageResponse>;
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
