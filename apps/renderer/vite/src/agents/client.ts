import { fetchAgentsRuntimeSnapshot } from "./runtime";

export type AgentInstanceState = "ready" | "degraded" | "unreachable";

export type SantiProviderFacts = {
  api: string;
  model: string;
  gateway_base_url: string | null;
};

export type AgentProfileSecretState = "available" | "missing";

export type AgentProfileSummary = {
  id: string;
  label: string;
  launch_profile: string;
  provider: SantiProviderFacts;
  secret_state: AgentProfileSecretState;
};

export type AgentProfileListResponse = {
  profiles: AgentProfileSummary[];
};

export type SantiProviderProbeState = "ready" | "degraded" | "unreachable";

export type SantiProviderProbeFacts = {
  state: SantiProviderProbeState;
  checked_url: string;
  http_status: number | null;
  detail: string | null;
};

export type SantiRuntimeFacts = {
  execution_root: string;
  runtime_root: string;
  standalone_sqlite_path: string | null;
};

export type SantiConfigFacts = {
  config_version: number;
  last_event_id: string;
  source: string;
  launch_profile: string | null;
  provider: SantiProviderFacts;
  runtime: SantiRuntimeFacts;
};

export type SantiCapabilityFacts = {
  health: boolean;
  sessions: boolean;
  soul: boolean;
  admin_hooks: boolean;
  streaming: boolean;
};

export type SantiServiceFacts = {
  api_version: string | null;
  service_name: string;
  service_version: string | null;
  mode: string | null;
  launch_profile: string | null;
  bind_addr: string | null;
  capabilities: SantiCapabilityFacts | null;
};

export type AgentProcessFacts = {
  pid: number;
  launched_by_agents: boolean;
};

export type AgentProcessStopFacts = {
  already_stopped: boolean;
  matched_pids: number[];
  stopped_pids: number[];
  forced_pids: number[];
  remaining_pids: number[];
};

export type AgentInstanceSnapshot = {
  id: string;
  agent_id: string;
  participant_id: string;
  delivery_endpoint_id: string;
  label: string;
  agent_kind: string;
  managed: boolean;
  active: boolean;
  state: AgentInstanceState;
  endpoint: string | null;
  profile: string | null;
  process: AgentProcessFacts | null;
  service: SantiServiceFacts | null;
  config: SantiConfigFacts | null;
  provider: SantiProviderFacts | null;
  provider_probe: SantiProviderProbeFacts | null;
  runtime: SantiRuntimeFacts | null;
  last_probe_at: string;
  detail: string | null;
};

export type AgentInstanceListResponse = {
  active_instance_id: string;
  instances: AgentInstanceSnapshot[];
};

export type AgentSelectionResponse = {
  active_instance_id: string;
};

export type AgentProviderProbeResponse = {
  instance_id: string;
  provider_probe: SantiProviderProbeFacts;
};

export type AgentInstanceActionResponse = {
  event_id: string;
  instance_id: string;
  action: "launch" | "stop";
  status: "completed";
  snapshot: AgentInstanceSnapshot;
  process_result: AgentProcessStopFacts | null;
  detail: string | null;
};

export type AgentProfileApplyResponse = {
  event_id: string;
  instance_id: string;
  profile_id: string;
  status: "applied";
  santi_event_id: string;
  config_version: number;
  snapshot: AgentInstanceSnapshot;
  detail: string | null;
};

export async function fetchAgentInstances() {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(`${snapshot.http_base_url}/api/v1/agents/instances`);

  if (!response.ok) {
    throw new Error(`Agents instance fetch failed: ${response.status}`);
  }

  return response.json() as Promise<AgentInstanceListResponse>;
}

export async function fetchAgentProfiles() {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(`${snapshot.http_base_url}/api/v1/agents/profiles`);

  if (!response.ok) {
    throw new Error(`Agents profile fetch failed: ${response.status}`);
  }

  return response.json() as Promise<AgentProfileListResponse>;
}

export async function selectAgentInstance(instanceId: string) {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(`${snapshot.http_base_url}/api/v1/agents/selection`, {
    method: "PUT",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify({ instance_id: instanceId }),
  });

  if (!response.ok) {
    throw new Error(`Agents instance selection failed: ${response.status}`);
  }

  return response.json() as Promise<AgentSelectionResponse>;
}

export async function probeAgentInstance(instanceId: string) {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/agents/instances/${encodeURIComponent(instanceId)}/probe`,
    {
      method: "POST",
    },
  );

  if (!response.ok) {
    throw new Error(`Agents instance probe failed: ${response.status}`);
  }

  return response.json() as Promise<AgentInstanceSnapshot>;
}

export async function probeAgentInstanceProvider(instanceId: string) {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/agents/instances/${encodeURIComponent(instanceId)}/provider/probe`,
    {
      method: "POST",
    },
  );

  if (!response.ok) {
    throw new Error(`Agents provider probe failed: ${response.status}`);
  }

  return response.json() as Promise<AgentProviderProbeResponse>;
}

export async function launchAgentInstance(instanceId: string) {
  return postAgentInstanceAction(instanceId, "launch");
}

export async function stopAgentInstance(instanceId: string) {
  return postAgentInstanceAction(instanceId, "stop");
}

export async function applyAgentProfile(instanceId: string, profileId: string) {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/agents/instances/${encodeURIComponent(instanceId)}/profiles/apply`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ profile_id: profileId }),
    },
  );

  if (!response.ok) {
    throw new Error(`Agents profile apply failed: ${response.status}`);
  }

  return response.json() as Promise<AgentProfileApplyResponse>;
}

async function postAgentInstanceAction(instanceId: string, action: "launch" | "stop") {
  const snapshot = await fetchAgentsRuntimeSnapshot();

  if (!snapshot.http_base_url) {
    throw new Error("Agents HTTP base URL unavailable");
  }

  const response = await fetch(
    `${snapshot.http_base_url}/api/v1/agents/instances/${encodeURIComponent(instanceId)}/${action}`,
    {
      method: "POST",
    },
  );

  if (!response.ok) {
    throw new Error(`Agents ${action} action failed: ${response.status}`);
  }

  return response.json() as Promise<AgentInstanceActionResponse>;
}
