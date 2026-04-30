<script setup lang="ts">
import { StimAppRoot, StimSplit } from "@stim-io/components";
import { computed, onMounted, ref } from "vue";

import MessagesPane from "./components/im/MessagesPane.vue";
import SessionDrawer from "./components/im/SessionDrawer.vue";
import {
  createChatMessage,
  initialLiveMessages,
  previewForContent,
  staticSessions,
  textContent,
} from "./components/im/sessionModel";
import type { ChatMessage, SessionSummary } from "./components/im/types";
import {
  sendFirstMessage,
  type FirstMessageResponse,
} from "./controller/client";
import {
  fetchControllerRuntimeSnapshot,
  hasTauriHostRuntime,
  type ControllerRuntimeSnapshot,
} from "./controller/runtime";

const draftText = ref("");
const targetEndpointId = ref("endpoint-b");
const controllerSnapshot = ref<ControllerRuntimeSnapshot | null>(null);
const sendResult = ref<FirstMessageResponse | null>(null);
const activeConversationId = ref<string | null>(null);
const liveMessages = ref<ChatMessage[]>(initialLiveMessages());
const activeSessionId = ref("live-controller");
const isSessionDrawerCollapsed = ref(false);
const sessionQuery = ref("");
const activeSessionScope = ref<"all" | "live" | "unread">("all");
const errorMessage = ref<string | null>(null);
const isLoading = ref(false);
const optimisticMessageId = ref<string | null>(null);

const controllerStatus = computed(
  () => controllerSnapshot.value?.state ?? "unavailable",
);
const controllerAttached = computed(() => hasTauriHostRuntime());
const controllerBaseUrl = computed(
  () => controllerSnapshot.value?.http_base_url ?? "not attached",
);

const sessions = computed<SessionSummary[]>(() => {
  const latestLiveMessage = liveMessages.value.at(-1);

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
      participantLabel: "AI",
      live: controllerAttached.value,
      messages: liveMessages.value,
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
  if (!controllerAttached.value) {
    return;
  }

  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
});

async function handleSend() {
  if (!activeSession.value.live || !draftText.value.trim()) {
    return;
  }

  errorMessage.value = null;
  isLoading.value = true;

  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
    const sentDraft = draftText.value;
    const pendingId = `pending-${Date.now()}`;
    optimisticMessageId.value = pendingId;
    liveMessages.value.push(
      createChatMessage(
        pendingId,
        "user",
        "You",
        "Now",
        textContent(sentDraft),
        {
          deliveryState: "sending",
        },
      ),
    );
    draftText.value = "";

    sendResult.value = await sendFirstMessage(
      sentDraft,
      targetEndpointId.value,
      activeConversationId.value,
    );
    const sendResultValue = sendResult.value;
    activeConversationId.value = sendResultValue.conversation_id;
    liveMessages.value = liveMessages.value.map((message) =>
      message.id === pendingId
        ? createChatMessage(
            `${sendResultValue.message_id}-user`,
            "user",
            "You",
            "Now",
            sendResultValue.final_sent_content,
            {
              deliveryState: "sent",
              metaLabel: "Delivered to controller",
            },
          )
        : message,
    );
    liveMessages.value.push(
      createChatMessage(
        `${sendResultValue.message_id}-assistant`,
        "assistant",
        "stim",
        "Now",
        sendResultValue.response_content,
        {
          metaLabel: "Controller reply",
        },
      ),
    );
    optimisticMessageId.value = null;
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    if (optimisticMessageId.value) {
      liveMessages.value = liveMessages.value.map((message) =>
        message.id === optimisticMessageId.value
          ? {
              ...message,
              deliveryState: "failed",
              metaLabel: "Retry after controller recovers",
            }
          : message,
      );
      optimisticMessageId.value = null;
    }
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isLoading.value = false;
  }
}

function handleNewConversation() {
  activeSessionId.value = "live-controller";
  activeConversationId.value = null;
  liveMessages.value = [];
  sendResult.value = null;
  errorMessage.value = null;
  optimisticMessageId.value = null;
  draftText.value = "";
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
        :active-conversation-id="activeConversationId"
        :controller-base-url="controllerBaseUrl"
        :controller-status="controllerStatus"
        :draft-text="draftText"
        :error-message="errorMessage"
        :is-loading="isLoading"
        :last-final-sent-text="sendResult?.final_sent_text ?? null"
        :last-response-source="sendResult?.response_text_source ?? null"
        :last-response-text="sendResult?.response_text ?? null"
        :session="activeSession"
        :target-endpoint-id="targetEndpointId"
        @send="handleSend"
        @update:draft-text="draftText = $event"
        @update:target-endpoint-id="targetEndpointId = $event"
      />
    </StimSplit>
  </StimAppRoot>
</template>
