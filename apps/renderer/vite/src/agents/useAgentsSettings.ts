import { computed, onMounted, onUnmounted, reactive, ref } from "vue";

import {
  applyAgentsEvent,
  createAgentsModel,
  dispatchAgentsEvent,
  type AgentsRendererEvent,
} from "./agentsModel";
import {
  applyAgentProfile,
  fetchAgentInstances,
  fetchAgentProfiles,
  launchAgentInstance,
  probeAgentInstance,
  probeAgentInstanceProvider,
  selectAgentInstance,
  stopAgentInstance,
  type SantiCapabilityFacts,
} from "./client";
import { fetchAgentsRuntimeSnapshot, hasTauriHostRuntime } from "./runtime";
import {
  fetchChatParticipantSelection,
  fetchRegisteredAgentInstances,
  fetchRegisteredParticipants,
  selectChatParticipant,
} from "../server/agents";
import { rendererEventbus } from "../events/rootEventbus";

export function useAgentsSettings() {
  const agentsModel = reactive(createAgentsModel());
  const unsubscribeAgentsEvents = rendererEventbus.subscribeNamespace(
    "agents",
    (event: AgentsRendererEvent) => {
      applyAgentsEvent(agentsModel, event);
    },
  );

  const agentsRuntime = computed(() => agentsModel.agentsRuntime);
  const instances = computed(() => agentsModel.instances);
  const profiles = computed(() => agentsModel.profiles);
  const registeredAgents = computed(() => agentsModel.registeredAgents);
  const registeredParticipants = computed(
    () => agentsModel.registeredParticipants,
  );
  const activeInstanceId = computed(() => agentsModel.activeInstanceId);
  const selectedParticipantId = computed(
    () => agentsModel.selectedParticipantId,
  );
  const errorMessage = computed(() => agentsModel.errorMessage);
  const registryErrorMessage = computed(() => agentsModel.registryErrorMessage);
  const isLoading = ref(false);
  const probingInstanceId = ref<string | null>(null);
  const probingProviderInstanceId = ref<string | null>(null);
  const selectingInstanceId = ref<string | null>(null);
  const selectingParticipantId = ref<string | null>(null);
  const managingInstanceId = ref<string | null>(null);
  const applyingProfileKey = ref<string | null>(null);

  const runtimeState = computed(
    () => agentsRuntime.value?.state ?? "unavailable",
  );
  const runtimeBaseUrl = computed(
    () => agentsRuntime.value?.http_base_url ?? "not attached",
  );

  onMounted(() => {
    void refreshAgents();
  });

  onUnmounted(() => {
    unsubscribeAgentsEvents();
  });

  async function refreshAgents() {
    if (!hasTauriHostRuntime()) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: "Attach the Tauri desktop host to inspect agents.",
      });
      return;
    }

    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    isLoading.value = true;
    try {
      const runtime = await fetchAgentsRuntimeSnapshot();
      const [instancesResponse, profilesResponse] = await Promise.all([
        fetchAgentInstances(),
        fetchAgentProfiles(),
      ]);
      dispatchAgentsEvent(rendererEventbus, "runtime.loaded", {
        snapshot: runtime,
      });
      dispatchAgentsEvent(rendererEventbus, "instances.loaded", {
        active_instance_id: instancesResponse.active_instance_id,
        instances: instancesResponse.instances,
      });
      dispatchAgentsEvent(rendererEventbus, "profiles.loaded", {
        profiles: profilesResponse.profiles,
      });
      await refreshRegisteredAgents();
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      isLoading.value = false;
    }
  }

  async function refreshRegisteredAgents() {
    try {
      const [agentsResponse, participantsResponse, selectionResponse] =
        await Promise.all([
          fetchRegisteredAgentInstances(),
          fetchRegisteredParticipants(),
          fetchChatParticipantSelection(),
        ]);
      dispatchAgentsEvent(rendererEventbus, "registry.loaded", {
        registered_agents: agentsResponse.instances,
        registered_participants: participantsResponse.participants,
        selected_participant_id: selectionResponse.selected_participant_id,
      });
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "registry-error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    }
  }

  async function probeInstance(instanceId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    probingInstanceId.value = instanceId;
    try {
      const snapshot = await probeAgentInstance(instanceId);
      dispatchAgentsEvent(rendererEventbus, "instance.snapshot-updated", {
        snapshot,
      });
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      probingInstanceId.value = null;
    }
  }

  async function probeProvider(instanceId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    probingProviderInstanceId.value = instanceId;
    try {
      await probeAgentInstanceProvider(instanceId);
      await refreshAgents();
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      probingProviderInstanceId.value = null;
    }
  }

  async function makeActive(instanceId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    selectingInstanceId.value = instanceId;
    try {
      const selection = await selectAgentInstance(instanceId);
      dispatchAgentsEvent(rendererEventbus, "selection.changed", {
        active_instance_id: selection.active_instance_id,
      });
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      selectingInstanceId.value = null;
    }
  }

  async function launchInstance(instanceId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    managingInstanceId.value = instanceId;
    try {
      const response = await launchAgentInstance(instanceId);
      dispatchAgentsEvent(rendererEventbus, "instance.snapshot-updated", {
        snapshot: response.snapshot,
      });
      await refreshRegisteredAgents();
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      managingInstanceId.value = null;
    }
  }

  async function stopInstance(instanceId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    managingInstanceId.value = instanceId;
    try {
      const response = await stopAgentInstance(instanceId);
      dispatchAgentsEvent(rendererEventbus, "instance.snapshot-updated", {
        snapshot: response.snapshot,
      });
      await refreshRegisteredAgents();
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      managingInstanceId.value = null;
    }
  }

  async function applyProfile(instanceId: string, profileId: string) {
    dispatchAgentsEvent(rendererEventbus, "error.changed", { message: null });
    applyingProfileKey.value = `${instanceId}:${profileId}`;
    try {
      const response = await applyAgentProfile(instanceId, profileId);
      dispatchAgentsEvent(rendererEventbus, "instance.snapshot-updated", {
        snapshot: response.snapshot,
      });
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      applyingProfileKey.value = null;
    }
  }

  async function selectParticipant(participantId: string) {
    dispatchAgentsEvent(rendererEventbus, "registry-error.changed", {
      message: null,
    });
    selectingParticipantId.value = participantId;
    try {
      const selection = await selectChatParticipant(participantId);
      dispatchAgentsEvent(rendererEventbus, "participant-selection.changed", {
        selected_participant_id: selection.selected_participant_id,
      });
      await refreshRegisteredAgents();
    } catch (error) {
      dispatchAgentsEvent(rendererEventbus, "registry-error.changed", {
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      selectingParticipantId.value = null;
    }
  }

  function profileApplyKey(instanceId: string, profileId: string) {
    return `${instanceId}:${profileId}`;
  }

  function enabledCapabilityLabels(
    capabilities: SantiCapabilityFacts | null | undefined,
  ) {
    if (!capabilities) {
      return "unknown";
    }

    const labels = Object.entries({
      health: capabilities.health,
      sessions: capabilities.sessions,
      soul: capabilities.soul,
      admin_hooks: capabilities.admin_hooks,
      streaming: capabilities.streaming,
    })
      .filter(([, enabled]) => enabled)
      .map(([name]) => name.replace("_", "-"))
      .join(", ");

    return labels || "none";
  }

  return {
    activeInstanceId,
    agentsRuntime,
    applyingProfileKey,
    applyProfile,
    enabledCapabilityLabels,
    errorMessage,
    instances,
    isLoading,
    launchInstance,
    makeActive,
    managingInstanceId,
    probeInstance,
    probeProvider,
    probingInstanceId,
    probingProviderInstanceId,
    profileApplyKey,
    profiles,
    refreshAgents,
    registeredAgents,
    registeredParticipants,
    registryErrorMessage,
    runtimeBaseUrl,
    runtimeState,
    selectParticipant,
    selectedParticipantId,
    selectingInstanceId,
    selectingParticipantId,
    stopInstance,
  };
}
