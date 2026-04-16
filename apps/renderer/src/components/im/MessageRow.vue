<script setup lang="ts">
import { StimAvatar, StimInline, StimStack, StimText } from "@stim-io/components";
import { computed } from "vue";

import MessageCard from "../MessageCard.vue";
import type { ChatMessage } from "./types";

const props = defineProps<{
  message: ChatMessage;
}>();

const isUser = computed(() => props.message.role === "user");
const avatarTone = computed(() => (isUser.value ? "accent" : "muted"));
const justifyClass = computed(() =>
  isUser.value ? "message-row message-row--user" : "message-row message-row--assistant",
);
const deliveryLabel = computed(() => {
  if (!isUser.value) {
    return props.message.metaLabel ?? null;
  }

  if (props.message.deliveryState === "sending") {
    return "Sending…";
  }

  if (props.message.deliveryState === "failed") {
    return "Failed to send";
  }

  return props.message.metaLabel ?? "Sent";
});
</script>

<template>
  <div :class="justifyClass">
    <StimInline :justify="isUser ? 'end' : 'start'" align="start" gap="sm">
      <StimAvatar
        v-if="!isUser"
        :label="message.author"
        :tone="avatarTone"
        size="md"
      />

      <StimStack class="message-row__content" gap="xs">
        <StimInline :justify="isUser ? 'end' : 'start'" gap="sm">
          <StimText as="span" size="label">{{ message.author }}</StimText>
          <StimText as="span" size="caption" tone="secondary">
            {{ message.sentAtLabel }}
          </StimText>
          <StimText
            v-if="deliveryLabel"
            as="span"
            :tone="message.deliveryState === 'failed' ? 'primary' : 'secondary'"
            size="caption"
          >
            {{ deliveryLabel }}
          </StimText>
        </StimInline>
        <MessageCard :role="message.role" :content="message.content" />
      </StimStack>

      <StimAvatar
        v-if="isUser"
        :label="message.author"
        :tone="avatarTone"
        size="md"
      />
    </StimInline>
  </div>
</template>

<style scoped>
.message-row {
  inline-size: 100%;
}

.message-row :deep(.stim-inline) {
  inline-size: 100%;
}

.message-row__content {
  max-inline-size: min(42rem, 100%);
}

.message-row--user .message-row__content {
  align-items: flex-end;
}
</style>
