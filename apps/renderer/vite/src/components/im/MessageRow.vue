<script setup lang="ts">
import { StimMessageRow } from "@stim-io/components";
import { computed } from "vue";

import MessageCard from "../MessageCard.vue";
import type { ChatMessage } from "./types";

// Dev-flavor labels emitted by the live chat model for observability
// ("Controller reply", "Delivered to controller", etc.) should not surface
// in the product UI — the bubble itself is proof of delivery. Only sending /
// failed transitions warrant a visible label.
const DEV_OBSERVABILITY_LABELS = new Set([
  "Sent",
  "Controller reply",
  "Controller reply streaming",
  "Delivered to controller",
  "Controller operation running",
]);

const props = defineProps<{
  message: ChatMessage;
}>();

const deliveryLabel = computed(() => {
  if (props.message.deliveryState === "sending") {
    return "Sending…";
  }
  if (props.message.deliveryState === "failed") {
    return "Failed to send";
  }
  const raw = props.message.metaLabel;
  if (!raw || DEV_OBSERVABILITY_LABELS.has(raw)) {
    return null;
  }
  return raw;
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
