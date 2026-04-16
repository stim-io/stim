<script setup lang="ts">
import {
  StimBadge,
  StimButton,
  StimInfoList,
  StimInline,
  StimInput,
  StimPane,
  StimStack,
  StimText,
} from "@stim-io/components";

import MessageRow from "./MessageRow.vue";
import type { SessionSummary } from "./types";

defineProps<{
  session: SessionSummary;
  draftText: string;
  targetEndpointId: string;
  isLoading: boolean;
  errorMessage: string | null;
  controllerStatus: string;
  controllerBaseUrl: string;
  activeConversationId: string | null;
  lastResponseText: string | null;
  lastResponseSource: string | null;
  lastFinalSentText: string | null;
}>();

const emit = defineEmits<{
  "update:draftText": [value: string];
  "update:targetEndpointId": [value: string];
  send: [];
}>();
</script>

<template>
  <StimPane grow padding="none" radius="none" border="none">
    <StimPane border="subtle" grow padding="none" radius="none">
      <StimStack gap="md" style="block-size: 100%">
        <StimPane border="subtle" padding="lg" radius="none">
          <StimInline justify="between" align="start">
            <StimStack gap="xs">
              <StimInline gap="sm" wrap>
                <StimText as="p" size="eyebrow" tone="secondary">messages</StimText>
                <StimBadge :tone="session.live ? 'accent' : 'muted'">
                  {{ session.live ? 'live' : 'mock' }}
                </StimBadge>
              </StimInline>
              <StimText as="h2" size="label">{{ session.title }}</StimText>
              <StimText as="p" size="caption" tone="secondary">
                {{ session.live ? 'Controller-backed desktop thread' : 'Static reference thread for layout pressure' }}
              </StimText>
            </StimStack>
            <StimStack align="end" gap="xs">
              <StimText as="span" size="label">{{ session.participantLabel }}</StimText>
              <StimText as="span" size="caption" tone="secondary">
                {{ controllerStatus }}
              </StimText>
            </StimStack>
          </StimInline>
        </StimPane>

        <StimPane
          class="messages-pane__thread"
          data-probe="landing-card"
          grow
          padding="md"
          radius="lg"
          scroll
          tone="muted"
        >
          <StimStack class="messages-pane__thread-stack" data-probe="chat-thread" gap="md">
            <StimText
              v-if="session.messages.length === 0"
              as="p"
              size="body"
              tone="secondary"
            >
              No messages yet.
            </StimText>
            <MessageRow v-for="message in session.messages" :key="message.id" :message="message" />
          </StimStack>
        </StimPane>

        <StimPane data-probe="landing-actions" padding="lg" radius="none" tone="default">
          <StimStack gap="sm">
            <StimStack gap="xs">
              <StimText as="p" size="label">Composer</StimText>
              <StimText as="p" size="caption" tone="secondary">
                Text-only for this phase. Business flow stays real only on the live controller thread.
              </StimText>
            </StimStack>
            <StimInput
              :model-value="draftText"
              class="messages-pane__input"
              data-probe="message-input"
              placeholder="Type a message"
              type="text"
              @update:model-value="emit('update:draftText', $event)"
            />
            <StimInput
              :model-value="targetEndpointId"
              class="messages-pane__input"
              data-probe="target-endpoint-input"
              :disabled="!session.live"
              placeholder="target endpoint"
              type="text"
              @update:model-value="emit('update:targetEndpointId', $event)"
            />
            <StimInline gap="sm" wrap>
              <StimButton
                :disabled="!session.live || isLoading"
                :label="isLoading ? 'Sending…' : 'Send message'"
                @click="emit('send')"
              />
              <StimText v-if="!session.live" as="span" size="caption" tone="secondary">
                Mock sessions are read-only in this slice.
              </StimText>
            </StimInline>
          </StimStack>
        </StimPane>

        <StimPane padding="lg" radius="none" tone="default">
          <StimInfoList class="messages-pane__debug" gap="sm">
          <div>
            <dt>Controller state</dt>
            <dd>{{ controllerStatus }}</dd>
          </div>
          <div>
            <dt>Controller URL</dt>
            <dd>{{ controllerBaseUrl }}</dd>
          </div>
          <div v-if="activeConversationId">
            <dt>Conversation</dt>
            <dd data-probe="active-conversation-id">{{ activeConversationId }}</dd>
          </div>
          <div v-if="lastResponseText">
            <dt>Response</dt>
            <dd data-probe="last-response-text">{{ lastResponseText }}</dd>
          </div>
          <div v-if="lastResponseSource">
            <dt>Response source</dt>
            <dd data-probe="last-response-source">{{ lastResponseSource }}</dd>
          </div>
          <div v-if="lastFinalSentText">
            <dt>Final sent text</dt>
            <dd data-probe="last-final-sent-text">{{ lastFinalSentText }}</dd>
          </div>
          <div v-if="errorMessage">
            <dt>Error</dt>
            <dd data-probe="last-error-message">{{ errorMessage }}</dd>
          </div>
          </StimInfoList>
        </StimPane>
      </StimStack>
    </StimPane>
  </StimPane>
</template>

<style scoped>
.messages-pane__thread {
  min-block-size: 0;
}

.messages-pane__thread-stack {
  min-block-size: 100%;
}

.messages-pane__input {
  inline-size: 100%;
}
</style>
