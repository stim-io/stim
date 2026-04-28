<script setup lang="ts">
import {
  StimMessageCardFrame,
  StimRichContent,
  StimStack,
} from "@stim-io/components";
import { computed } from "vue";

import type {
  MessageContent,
  MessageLayoutHint,
  MessagePart,
} from "../controller/client";

const props = defineProps<{
  role: "user" | "assistant" | "system";
  content: MessageContent;
}>();

const layout = computed(() => normalizeLayout(props.content.layout_hint));

function normalizeLayout(layoutHint: MessageLayoutHint | null) {
  return {
    layoutFamily: layoutHint?.layout_family === "card" ? "card" : "bubble",
    verticalPressure:
      layoutHint?.vertical_pressure === "compact" ||
      layoutHint?.vertical_pressure === "expand" ||
      layoutHint?.vertical_pressure === "scroll"
        ? layoutHint.vertical_pressure
        : "none",
    minHeightPx: layoutHint?.min_height_px ?? null,
    maxHeightPx: layoutHint?.max_height_px ?? null,
  } as const;
}

function partKey(part: MessagePart, index: number) {
  return `${part.kind}-${index}`;
}
</script>

<template>
  <StimMessageCardFrame
    data-probe="chat-bubble"
    :data-role="role"
    :role-tone="role"
    :layout-family="layout.layoutFamily"
    :vertical-pressure="layout.verticalPressure"
    :min-height-px="layout.minHeightPx"
    :max-height-px="layout.maxHeightPx"
  >
    <StimStack gap="sm">
      <StimRichContent
        v-for="(part, index) in content.parts"
        :key="partKey(part, index)"
        :kind="
          part.kind === 'raw_html'
            ? 'raw-html'
            : part.kind === 'stim_dom_fragment'
              ? 'stim-dom-fragment'
              : 'text'
        "
        :text="part.kind === 'text' ? part.text : undefined"
        :tree="part.kind === 'stim_dom_fragment' ? part.tree : undefined"
        :html="part.kind === 'raw_html' ? part.html : undefined"
      />
    </StimStack>
  </StimMessageCardFrame>
</template>
