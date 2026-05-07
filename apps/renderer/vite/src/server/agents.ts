export type RegisteredAgentStatus =
  | "ready"
  | "degraded"
  | "unreachable"
  | "offline";

export type RegisteredAgentInstance = {
  agent_id: string;
  instance_id: string;
  participant_id: string;
  delivery_endpoint_id: string | null;
  label: string;
  agent_kind: string;
  endpoint: string | null;
  profile: string | null;
  capabilities: string[];
  status: RegisteredAgentStatus;
  detail: string | null;
  registered_at: string;
  last_seen_at: string;
  last_event_id: string;
};

export type RegisteredAgentListResponse = {
  instances: RegisteredAgentInstance[];
};

export type RegisteredParticipantStatus =
  | "ready"
  | "degraded"
  | "unreachable"
  | "offline";

export type ParticipantSource = {
  source_kind: string;
  agent_id: string | null;
  instance_id: string | null;
};

export type ParticipantDeliveryTarget = {
  endpoint_id: string;
  address: string | null;
  carrier_kind: string | null;
};

export type RegisteredParticipant = {
  participant_id: string;
  display_label: string;
  markers: string[];
  capabilities: string[];
  status: RegisteredParticipantStatus;
  source: ParticipantSource;
  delivery_target: ParticipantDeliveryTarget | null;
  detail: string | null;
  registered_at: string;
  last_seen_at: string;
  last_event_id: string;
};

export type RegisteredParticipantListResponse = {
  participants: RegisteredParticipant[];
};

export type ChatParticipantSelectionResponse = {
  selected_participant_id: string | null;
  participant: RegisteredParticipant | null;
  last_event_id: string | null;
};

const DEFAULT_SERVER_BASE_URL = "http://127.0.0.1:18083";

export async function fetchRegisteredAgentInstances() {
  const response = await fetch(
    `${stimServerBaseUrl()}/api/v1/agents/instances`,
  );

  if (!response.ok) {
    throw new Error(`Registered agent fetch failed: ${response.status}`);
  }

  return response.json() as Promise<RegisteredAgentListResponse>;
}

export async function fetchRegisteredParticipants() {
  const response = await fetch(`${stimServerBaseUrl()}/api/v1/participants`);

  if (!response.ok) {
    throw new Error(`Registered participant fetch failed: ${response.status}`);
  }

  return response.json() as Promise<RegisteredParticipantListResponse>;
}

export async function fetchChatParticipantSelection() {
  const response = await fetch(
    `${stimServerBaseUrl()}/api/v1/chat/participant-selection`,
  );

  if (!response.ok) {
    throw new Error(`Chat participant selection fetch failed: ${response.status}`);
  }

  return response.json() as Promise<ChatParticipantSelectionResponse>;
}

export async function selectChatParticipant(participantId: string) {
  const response = await fetch(
    `${stimServerBaseUrl()}/api/v1/chat/participant-selection`,
    {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ participant_id: participantId }),
    },
  );

  if (!response.ok) {
    throw new Error(`Chat participant selection failed: ${response.status}`);
  }

  return response.json() as Promise<ChatParticipantSelectionResponse>;
}

function stimServerBaseUrl() {
  return (
    import.meta.env.VITE_STIM_SERVER_BASE_URL ?? DEFAULT_SERVER_BASE_URL
  ).replace(/\/+$/, "");
}
