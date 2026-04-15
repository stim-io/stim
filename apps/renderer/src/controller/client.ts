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
  final_message_version: number;
  response_text: string;
  response_text_source: string;
  sent_envelope_id: string;
  response_envelope_id: string;
  receipt_result: string;
  receipt_detail: string | null;
  lifecycle_trace: LifecycleTraceStep[];
  lifecycle_proof: LifecycleProof;
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

  const response = await fetch(`${snapshot.http_base_url}/api/v1/messages/roundtrip`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify({
      text,
      target_endpoint_id: targetEndpointId,
      conversation_id: conversationId ?? null
    })
  });

  if (!response.ok) {
    throw new Error(`Controller roundtrip failed: ${response.status}`);
  }

  return response.json() as Promise<FirstMessageResponse>;
}
