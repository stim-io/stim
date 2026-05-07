import type {
  RegisteredAgentInstance,
  RegisteredParticipant,
} from "../server/agents";
import {
  type RendererEventBus,
  type RendererEventEnvelope,
  rendererEvent,
} from "../events/eventbus";

import type {
  AgentInstanceSnapshot,
  AgentProfileSummary,
} from "./client";
import type { AgentsRuntimeSnapshot } from "./runtime";

type AgentsEventMap = {
  "runtime.loaded": {
    snapshot: AgentsRuntimeSnapshot;
  };
  "instances.loaded": {
    active_instance_id: string | null;
    instances: AgentInstanceSnapshot[];
  };
  "profiles.loaded": {
    profiles: AgentProfileSummary[];
  };
  "registry.loaded": {
    registered_agents: RegisteredAgentInstance[];
    registered_participants: RegisteredParticipant[];
    selected_participant_id: string | null;
  };
  "instance.snapshot-updated": {
    snapshot: AgentInstanceSnapshot;
  };
  "selection.changed": {
    active_instance_id: string | null;
  };
  "participant-selection.changed": {
    selected_participant_id: string | null;
  };
  "error.changed": {
    message: string | null;
  };
  "registry-error.changed": {
    message: string | null;
  };
};

export type AgentsEventKey = keyof AgentsEventMap;

export type AgentsRendererEvent<Key extends AgentsEventKey = AgentsEventKey> = {
  [EventKey in AgentsEventKey]: RendererEventEnvelope<
    "agents",
    EventKey,
    AgentsEventMap[EventKey]
  >;
}[Key];

export type AgentsModel = {
  agentsRuntime: AgentsRuntimeSnapshot | null;
  instances: AgentInstanceSnapshot[];
  profiles: AgentProfileSummary[];
  registeredAgents: RegisteredAgentInstance[];
  registeredParticipants: RegisteredParticipant[];
  activeInstanceId: string | null;
  selectedParticipantId: string | null;
  errorMessage: string | null;
  registryErrorMessage: string | null;
};

export function createAgentsModel(): AgentsModel {
  return {
    agentsRuntime: null,
    instances: [],
    profiles: [],
    registeredAgents: [],
    registeredParticipants: [],
    activeInstanceId: null,
    selectedParticipantId: null,
    errorMessage: null,
    registryErrorMessage: null,
  };
}

export function dispatchAgentsEvent<Key extends AgentsEventKey>(
  eventbus: RendererEventBus,
  eventkey: Key,
  payload: AgentsEventMap[Key],
) {
  eventbus.dispatch(rendererEvent("agents", eventkey, payload));
}

export function applyAgentsEvent(
  model: AgentsModel,
  event: AgentsRendererEvent,
) {
  switch (event.eventkey) {
    case "runtime.loaded":
      model.agentsRuntime = event.payload.snapshot;
      return;
    case "instances.loaded":
      model.activeInstanceId = event.payload.active_instance_id;
      model.instances = event.payload.instances;
      return;
    case "profiles.loaded":
      model.profiles = event.payload.profiles;
      return;
    case "registry.loaded":
      model.registeredAgents = event.payload.registered_agents;
      model.registeredParticipants = event.payload.registered_participants;
      model.selectedParticipantId = event.payload.selected_participant_id;
      model.registryErrorMessage = null;
      return;
    case "instance.snapshot-updated":
      if (event.payload.snapshot.active) {
        model.activeInstanceId = event.payload.snapshot.id;
      }
      model.instances = model.instances.map((instance) =>
        instance.id === event.payload.snapshot.id
          ? event.payload.snapshot
          : instance,
      );
      return;
    case "selection.changed":
      model.activeInstanceId = event.payload.active_instance_id;
      model.instances = model.instances.map((instance) => ({
        ...instance,
        active: instance.id === event.payload.active_instance_id,
      }));
      return;
    case "participant-selection.changed":
      model.selectedParticipantId = event.payload.selected_participant_id;
      return;
    case "error.changed":
      model.errorMessage = event.payload.message;
      return;
    case "registry-error.changed":
      if (event.payload.message) {
        model.registeredAgents = [];
        model.registeredParticipants = [];
        model.selectedParticipantId = null;
      }
      model.registryErrorMessage = event.payload.message;
      return;
  }
}
