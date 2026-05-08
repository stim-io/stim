<script setup lang="ts">
import { StimAppRoot, StimSplit } from "@stim-io/components";
import { computed, onMounted, reactive, ref, watch } from "vue";

import {
  applyChatEvent,
  createLiveChatModel,
  dispatchChatEvent,
  type ChatRendererEvent,
} from "./components/im/liveChatModel";
import MessagesPane from "./components/im/MessagesPane.vue";
import SessionDrawer from "./components/im/SessionDrawer.vue";
import { previewForContent, staticSessions } from "./components/im/sessionModel";
import type { SessionSummary } from "./components/im/types";
import {
  fetchConversationTranscript,
  sendTextOperation,
} from "./controller/client";
import {
  fetchControllerRuntimeSnapshot,
  hasTauriHostRuntime,
  type ControllerRuntimeSnapshot,
} from "./controller/runtime";
import { rendererEventbus } from "./events/rootEventbus";
import {
  fetchChatParticipantSelection,
  fetchRegisteredParticipants,
  selectChatParticipant,
  type RegisteredParticipant,
} from "@stim-io/agents-client";

const activeConversationStorageKey = "stim.activeConversationId";
const storedConversationId = readStoredConversationId();
const liveChatModel = reactive(createLiveChatModel(storedConversationId));
rendererEventbus.subscribeNamespace("chat", (event: ChatRendererEvent) => {
  applyChatEvent(liveChatModel, event);
});

const draftText = ref("");
const targetEndpointId = ref("endpoint-b");
const controllerSnapshot = ref<ControllerRuntimeSnapshot | null>(null);
const activeSessionId = ref("live-controller");
const isSessionDrawerCollapsed = ref(false);
const sessionQuery = ref("");
const activeSessionScope = ref<"all" | "live" | "unread">("all");
const participantErrorMessage = ref<string | null>(null);
const isLoading = ref(false);
const isParticipantSelecting = ref(false);
const registeredParticipants = ref<RegisteredParticipant[]>([]);
const selectedParticipantId = ref<string | null>(null);

const controllerStatus = computed(
  () => controllerSnapshot.value?.state ?? "unavailable",
);
const controllerAttached = computed(() => hasTauriHostRuntime());
const controllerBaseUrl = computed(
  () => controllerSnapshot.value?.http_base_url ?? "not attached",
);
const selectedParticipant = computed(
  () =>
    registeredParticipants.value.find(
      (participant) =>
        participant.participant_id === selectedParticipantId.value,
    ) ?? null,
);

const sessions = computed<SessionSummary[]>(() => {
  const latestLiveMessage = liveChatModel.messages.at(-1);

  return [
    {
      id: "live-controller",
      title: "Controller live thread",
      preview: controllerAttached.value
        ? latestLiveMessage
          ? previewForContent(latestLiveMessage.content)
          : "Start a real text roundtrip"
        : "Attach the Tauri desktop host to enable live controller roundtrips.",
      activityLabel: controllerStatus.value,
      unreadCount: 0,
      participantLabel: selectedParticipant.value?.display_label ?? "AI",
      live: controllerAttached.value,
      messages: liveChatModel.messages,
      toolActivities: liveChatModel.toolActivities,
    },
    ...staticSessions,
  ];
});

const visibleSessions = computed(() => {
  const normalizedQuery = sessionQuery.value.trim().toLowerCase();

  return sessions.value.filter((session) => {
    const matchesScope =
      activeSessionScope.value === "all"
        ? true
        : activeSessionScope.value === "live"
          ? session.live
          : session.unreadCount > 0;

    if (!matchesScope) {
      return false;
    }

    if (!normalizedQuery) {
      return true;
    }

    return [session.title, session.preview, session.participantLabel]
      .join(" ")
      .toLowerCase()
      .includes(normalizedQuery);
  });
});

const activeSession = computed(
  () =>
    visibleSessions.value.find(
      (session) => session.id === activeSessionId.value,
    ) ??
    sessions.value.find((session) => session.id === activeSessionId.value) ??
    visibleSessions.value[0] ??
    sessions.value[0],
);

onMounted(async () => {
  void refreshParticipants();

  if (!controllerAttached.value) {
    return;
  }

  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    dispatchChatEvent(rendererEventbus, "error.changed", {
      message: error instanceof Error ? error.message : String(error),
    });
  }

  if (liveChatModel.activeConversationId) {
    const conversationId = liveChatModel.activeConversationId;
    try {
      await reloadConversation(conversationId, { applyOnlyIfActive: true });
    } catch (error) {
      if (liveChatModel.activeConversationId === conversationId) {
        dispatchChatEvent(rendererEventbus, "conversation.load-failed", {
          conversation_id: conversationId,
          message: error instanceof Error ? error.message : String(error),
        });
      }
    }
  }
});

watch(
  () => liveChatModel.activeConversationId,
  (conversationId) => {
    if (conversationId) {
      storeConversationId(conversationId);
    } else {
      clearStoredConversationId();
    }
  },
);

async function refreshParticipants() {
  try {
    const [participantsResponse, selectionResponse] = await Promise.all([
      fetchRegisteredParticipants(),
      fetchChatParticipantSelection(),
    ]);
    registeredParticipants.value = participantsResponse.participants;
    selectedParticipantId.value = selectionResponse.selected_participant_id;
    participantErrorMessage.value = null;
  } catch (error) {
    registeredParticipants.value = [];
    selectedParticipantId.value = null;
    participantErrorMessage.value =
      error instanceof Error ? error.message : String(error);
  }
}

async function handleSend() {
  if (!activeSession.value.live || !draftText.value.trim()) {
    return;
  }

  dispatchChatEvent(rendererEventbus, "error.changed", { message: null });
  isLoading.value = true;
  let roundtripCompleted = false;

  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
    const sentDraft = draftText.value;
    const pendingId = `pending-${Date.now()}`;
    dispatchChatEvent(rendererEventbus, "message.optimistic-created", {
      message_id: pendingId,
      text: sentDraft,
    });
    draftText.value = "";

    const terminalEvent = await sendTextOperation(
      sentDraft,
      targetEndpointId.value,
      selectedParticipantId.value,
      liveChatModel.activeConversationId,
      (event) => {
        dispatchChatEvent(rendererEventbus, "controller.operation-event", {
          event,
        });
      },
    );
    roundtripCompleted = true;
    if (!terminalEvent.snapshot && terminalEvent.conversation_id) {
      await reloadConversation(terminalEvent.conversation_id);
    }
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    if (liveChatModel.optimisticMessageId && !roundtripCompleted) {
      dispatchChatEvent(rendererEventbus, "message.delivery-failed", {
        message_id: liveChatModel.optimisticMessageId,
        meta_label: "Retry after controller recovers",
      });
    }
    dispatchChatEvent(rendererEventbus, "error.changed", {
      message: error instanceof Error ? error.message : String(error),
    });
  } finally {
    isLoading.value = false;
  }
}

function handleNewConversation() {
  activeSessionId.value = "live-controller";
  dispatchChatEvent(rendererEventbus, "conversation.reset", {});
  draftText.value = "";
}

async function handleSelectParticipant(participantId: string) {
  participantErrorMessage.value = null;
  isParticipantSelecting.value = true;
  try {
    const selection = await selectChatParticipant(participantId);
    selectedParticipantId.value = selection.selected_participant_id;
    await refreshParticipants();
  } catch (error) {
    participantErrorMessage.value =
      error instanceof Error ? error.message : String(error);
  } finally {
    isParticipantSelecting.value = false;
  }
}

async function reloadConversation(
  conversationId: string,
  options: { applyOnlyIfActive?: boolean } = {},
) {
  const transcript = await fetchConversationTranscript(conversationId);
  if (
    options.applyOnlyIfActive &&
    liveChatModel.activeConversationId !== conversationId
  ) {
    return;
  }

  dispatchChatEvent(rendererEventbus, "transcript.loaded", {
    conversation_id: transcript.conversation_id,
    messages: transcript.messages,
    tool_activities: transcript.tool_activities,
  });
}

function readStoredConversationId() {
  try {
    return window.localStorage.getItem(activeConversationStorageKey);
  } catch {
    return null;
  }
}

function storeConversationId(conversationId: string) {
  try {
    window.localStorage.setItem(activeConversationStorageKey, conversationId);
  } catch {
    // Local storage is a convenience for reload continuity, not runtime truth.
  }
}

function clearStoredConversationId() {
  try {
    window.localStorage.removeItem(activeConversationStorageKey);
  } catch {
    // Local storage is a convenience for reload continuity, not runtime truth.
  }
}
</script>

<template>
  <StimAppRoot data-probe="landing-shell">
    <StimSplit gap="none">
      <SessionDrawer
        :active-session-id="activeSession.id"
        :collapsed="isSessionDrawerCollapsed"
        :controller-status="controllerStatus"
        :active-scope="activeSessionScope"
        :session-query="sessionQuery"
        :sessions="visibleSessions"
        @new-conversation="handleNewConversation"
        @select="activeSessionId = $event"
        @toggle-collapse="isSessionDrawerCollapsed = !isSessionDrawerCollapsed"
        @update:active-scope="activeSessionScope = $event"
        @update:session-query="sessionQuery = $event"
      />

      <MessagesPane
        :active-conversation-id="liveChatModel.activeConversationId"
        :controller-base-url="controllerBaseUrl"
        :controller-status="controllerStatus"
        :draft-text="draftText"
        :error-message="liveChatModel.errorMessage"
        :is-loading="isLoading"
        :last-final-sent-text="liveChatModel.lastFinalSentText"
        :last-response-source="liveChatModel.lastResponseSource"
        :last-response-text="liveChatModel.lastResponseText"
        :is-participant-selecting="isParticipantSelecting"
        :participant-error-message="participantErrorMessage"
        :registered-participants="registeredParticipants"
        :selected-participant-id="selectedParticipantId"
        :session="activeSession"
        :target-endpoint-id="targetEndpointId"
        @select-participant="handleSelectParticipant"
        @send="handleSend"
        @update:draft-text="draftText = $event"
        @update:target-endpoint-id="targetEndpointId = $event"
      />
    </StimSplit>
  </StimAppRoot>
</template>
