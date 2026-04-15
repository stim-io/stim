<script setup lang="ts">
import {
  StimButton,
  StimInfoList,
  StimInput,
  StimStack,
  StimSurface,
  StimText,
  StimViewportStage,
} from "@stim-io/components";
import { computed, onMounted, ref } from "vue";

import MessageCard from "./components/MessageCard.vue";
import {
  type MessageContent,
  sendFirstMessage,
  type FirstMessageResponse,
} from "./controller/client";
import {
  fetchControllerRuntimeSnapshot,
  type ControllerRuntimeSnapshot,
} from "./controller/runtime";

const draftText = ref("hello from stim ui");
const targetEndpointId = ref("endpoint-b");
const controllerSnapshot = ref<ControllerRuntimeSnapshot | null>(null);
const sendResult = ref<FirstMessageResponse | null>(null);
const activeConversationId = ref<string | null>(null);
const chatHistory = ref<
  Array<{ id: string; role: "user" | "assistant"; content: MessageContent }>
>([]);
const errorMessage = ref<string | null>(null);
const isLoading = ref(false);

const controllerStatus = computed(
  () => controllerSnapshot.value?.state ?? "unavailable",
);
const controllerBaseUrl = computed(
  () => controllerSnapshot.value?.http_base_url ?? "not attached",
);

onMounted(async () => {
  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
});

async function handleSend() {
  if (!draftText.value.trim()) {
    return;
  }

  errorMessage.value = null;
  isLoading.value = true;

  try {
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
    const sentDraft = draftText.value;
    sendResult.value = await sendFirstMessage(
      sentDraft,
      targetEndpointId.value,
      activeConversationId.value,
    );
    activeConversationId.value = sendResult.value.conversation_id;
    chatHistory.value.push(
      {
        id: `${sendResult.value.message_id}-user`,
        role: "user",
        content: sendResult.value.final_sent_content,
      },
      {
        id: `${sendResult.value.message_id}-assistant`,
        role: "assistant",
        content: sendResult.value.response_content,
      },
    );
    draftText.value = "";
    controllerSnapshot.value = await fetchControllerRuntimeSnapshot();
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isLoading.value = false;
  }
}

function handleNewConversation() {
  activeConversationId.value = null;
  chatHistory.value = [];
  sendResult.value = null;
  errorMessage.value = null;
  draftText.value = "hello from stim ui";
}
</script>

<template>
  <StimViewportStage class="landing-shell" data-probe="landing-shell">
    <StimSurface
      class="landing-card"
      data-probe="landing-card"
      tone="elevated"
      padding="lg"
      radius="lg"
    >
      <StimStack gap="lg">
        <StimStack gap="sm">
          <StimText as="p" size="eyebrow" tone="secondary">stim</StimText>
          <StimText as="h1" class="landing-title" data-probe="landing-title" size="display">
            Agent-native messaging, starting with a strict desktop landing.
          </StimText>
          <StimText as="p" class="landing-copy" tone="secondary" size="body">
            This rough proof uses Tauri IPC only for controller discovery/status
            and uses controller-local HTTP for a crude but real multi-turn chat
            loop.
          </StimText>
        </StimStack>

        <StimStack class="chat-thread" data-probe="chat-thread" gap="md">
          <StimText
            v-if="chatHistory.length === 0"
            as="p"
            tone="secondary"
            size="body"
          >
            No messages yet.
          </StimText>
          <div v-for="entry in chatHistory" :key="entry.id">
            <MessageCard :role="entry.role" :content="entry.content" />
          </div>
        </StimStack>

        <StimStack class="landing-actions" data-probe="landing-actions" gap="sm">
          <StimInput
            v-model="draftText"
            class="message-input"
            data-probe="message-input"
            type="text"
          />
          <StimInput
            v-model="targetEndpointId"
            class="message-input"
            data-probe="target-endpoint-input"
            type="text"
          />
          <StimButton
            :label="isLoading ? 'Sending…' : 'Send message'"
            @click="handleSend"
          />
          <StimButton label="New conversation" @click="handleNewConversation" />
        </StimStack>

        <StimInfoList class="debug-panel" gap="sm">
          <div>
            <dt>Controller state</dt>
            <dd>{{ controllerStatus }}</dd>
          </div>
          <div>
            <dt>Controller URL</dt>
            <dd>{{ controllerBaseUrl }}</dd>
          </div>
          <div v-if="sendResult">
            <dt>Conversation</dt>
            <dd data-probe="active-conversation-id">
              {{ sendResult.conversation_id }}
            </dd>
          </div>
          <div v-if="sendResult">
            <dt>Message</dt>
            <dd>{{ sendResult.message_id }}</dd>
          </div>
          <div v-if="sendResult">
            <dt>Target endpoint</dt>
            <dd>{{ sendResult.target_endpoint_id }}</dd>
          </div>
          <div v-if="sendResult">
            <dt>Response</dt>
            <dd data-probe="last-response-text">
              {{ sendResult.response_text }}
            </dd>
          </div>
          <div v-if="sendResult">
            <dt>Response source</dt>
            <dd data-probe="last-response-source">
              {{ sendResult.response_text_source }}
            </dd>
          </div>
          <div v-if="sendResult">
            <dt>Final sent text</dt>
            <dd data-probe="last-final-sent-text">
              {{ sendResult.final_sent_text }}
            </dd>
          </div>
          <div v-if="sendResult">
            <dt>Final version</dt>
            <dd>{{ sendResult.final_message_version }}</dd>
          </div>
          <div v-if="sendResult">
            <dt>Receipt</dt>
            <dd>{{ sendResult.receipt_result }}</dd>
          </div>
          <div v-if="sendResult?.receipt_detail">
            <dt>Receipt detail</dt>
            <dd>{{ sendResult.receipt_detail }}</dd>
          </div>
          <template v-if="sendResult?.lifecycle_trace?.length">
            <div
              v-for="step in sendResult.lifecycle_trace"
              :key="`${step.operation}-${step.sent_envelope_id}`"
            >
              <dt>Lifecycle {{ step.operation }}</dt>
              <dd>
                sent={{ step.sent_envelope_id }} ack={{
                  step.ack_envelope_id
                }}
                v={{ step.ack_version }} source={{ step.response_text_source }}
              </dd>
            </div>
          </template>
          <template v-if="sendResult?.lifecycle_proof">
            <div>
              <dt>Proof versions</dt>
              <dd>
                create={{
                  sendResult.lifecycle_proof.create_ack_version
                }}
                patch={{ sendResult.lifecycle_proof.patch_ack_version }} fix={{
                  sendResult.lifecycle_proof.fix_ack_version
                }}
                final={{ sendResult.lifecycle_proof.final_message_version }}
              </dd>
            </div>
            <div>
              <dt>Proof final text</dt>
              <dd>
                expected={{
                  sendResult.lifecycle_proof.expected_final_text
                }}
                observed={{ sendResult.lifecycle_proof.controller_final_text }}
              </dd>
            </div>
            <div>
              <dt>Proof checks</dt>
              <dd>
                versions={{
                  sendResult.lifecycle_proof.version_progression_valid
                }}
                text={{
                  sendResult.lifecycle_proof.final_text_matches_expected
                }}
              </dd>
            </div>
          </template>
          <div v-if="errorMessage">
            <dt>Error</dt>
            <dd data-probe="last-error-message">{{ errorMessage }}</dd>
          </div>
        </StimInfoList>
      </StimStack>
    </StimSurface>
  </StimViewportStage>
</template>
