<script setup lang="ts">
import { StimMessageRow } from "@stim-io/components";
import { computed } from "vue";

import MessageCard from "../MessageCard.vue";
import type { ChatMessage } from "./types";

const props = defineProps<{
  message: ChatMessage;
}>();

const isUser = computed(() => props.message.role === "user");
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

const deliveryTone = computed(() =>
  props.message.deliveryState === "failed" ? "danger" : "neutral",
);
</script>

<template>
  <StimMessageRow
    :author="message.author"
    :sent-at-label="message.sentAtLabel"
    :role="message.role"
    :meta-label="deliveryLabel"
    :meta-tone="deliveryTone"
  >
    <MessageCard :role="message.role" :content="message.content" />
  </StimMessageRow>
</template>
