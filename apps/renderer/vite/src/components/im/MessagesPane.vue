<script setup lang="ts">
import {
  StimBadge,
  StimButton,
  StimComposer,
  StimDisclosure,
  StimInline,
  StimInput,
  StimPane,
  StimStack,
  StimText,
} from "@stim-io/components";
import {
  computed,
  nextTick,
  ref,
  watch,
  type ComponentPublicInstance,
} from "vue";

import MessageRow from "./MessageRow.vue";
import type { SessionSummary } from "./types";
import type { RegisteredParticipant } from "../../server/agents";

const props = defineProps<{
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
  registeredParticipants: RegisteredParticipant[];
  selectedParticipantId: string | null;
  participantErrorMessage: string | null;
  isParticipantSelecting: boolean;
}>();

const threadPaneRef = ref<ComponentPublicInstance | HTMLElement | null>(null);

const canSend = computed(
  () =>
    props.session.live &&
    !props.isLoading &&
    props.draftText.trim().length > 0,
);
const selectedParticipant = computed(
  () =>
    props.registeredParticipants.find(
      (participant) =>
        participant.participant_id === props.selectedParticipantId,
    ) ?? null,
);
const headerSubtitle = computed(() =>
  props.session.live ? "Live thread" : "Mock reference thread",
);

const emit = defineEmits<{
  "update:draftText": [value: string];
  "update:targetEndpointId": [value: string];
  selectParticipant: [participantId: string];
  send: [];
}>();

watch(
  () => [props.session.id, props.session.messages.length],
  async () => {
    await nextTick();
    const candidate = threadPaneRef.value;
    const element =
      candidate instanceof HTMLElement ? candidate : candidate?.$el;

    if (element instanceof HTMLElement) {
      element.scrollTop = element.scrollHeight;
    }
  },
  { flush: "post" },
);
</script>

<template>
  <StimPane grow padding="none" radius="none" border="none">
    <StimStack gap="none" grow full-block>
      <StimPane
        border="none"
        padding="md"
        radius="none"
        tone="default"
      >
        <StimInline justify="between" align="center" gap="md">
          <StimStack gap="xs">
            <StimInline gap="sm" align="center" wrap>
              <StimText
                as="h2"
                size="heading-sm"
                tone="primary"
              >
                {{ session.title }}
              </StimText>
              <StimBadge
                :tone="session.live ? 'soft' : 'muted'"
                size="sm"
              >
                {{ session.live ? "live" : "mock" }}
              </StimBadge>
            </StimInline>
            <StimText as="p" size="caption" tone="tertiary">
              {{ headerSubtitle }} · via
              {{
                selectedParticipant?.display_label ?? session.participantLabel
              }}
            </StimText>
          </StimStack>
        </StimInline>
      </StimPane>

      <StimPane
        ref="threadPaneRef"
        data-probe="landing-card"
        grow
        padding="lg"
        radius="none"
        scroll
        tone="default"
        border="none"
      >
        <StimStack data-probe="chat-thread" gap="md">
          <StimText
            v-if="session.messages.length === 0"
            as="p"
            size="body"
            tone="tertiary"
          >
            No messages yet.
          </StimText>
          <MessageRow
            v-for="message in session.messages"
            :key="message.id"
            :message="message"
          />
        </StimStack>
      </StimPane>

      <StimPane
        data-probe="landing-actions"
        border="none"
        padding="md"
        radius="none"
        tone="default"
      >
        <StimStack gap="sm">
          <StimComposer
            :model-value="draftText"
            data-probe="message-input"
            placeholder="Send a message"
            @update:model-value="emit('update:draftText', $event)"
            :is-sending="isLoading"
            :can-send="canSend"
            send-label="Send"
            sending-label="Sending…"
            @send="emit('send')"
          />
          <StimText
            v-if="errorMessage"
            as="p"
            data-probe="last-error-message"
            size="caption"
            tone="danger"
          >
            {{ errorMessage }}
          </StimText>
          <StimDisclosure
            summary="Delivery"
            :caption="
              selectedParticipant?.display_label ?? session.participantLabel
            "
          >
            <StimStack gap="xs">
              <StimText as="p" size="caption" tone="tertiary">
                Chat participant
              </StimText>
              <StimInline gap="sm" wrap>
                <StimButton
                  v-for="participant in registeredParticipants"
                  :key="participant.participant_id"
                  :disabled="
                    isParticipantSelecting ||
                    !session.live ||
                    participant.participant_id === selectedParticipantId
                  "
                  :label="participant.display_label"
                  :pressed="
                    participant.participant_id === selectedParticipantId
                  "
                  data-probe="message-participant-select-button"
                  size="sm"
                  variant="ghost"
                  @click="
                    emit('selectParticipant', participant.participant_id)
                  "
                />
              </StimInline>
              <StimText
                v-if="participantErrorMessage"
                as="p"
                size="caption"
                tone="danger"
              >
                {{ participantErrorMessage }}
              </StimText>
              <StimInput
                :model-value="targetEndpointId"
                data-probe="target-endpoint-input"
                :disabled="!session.live"
                placeholder="target endpoint"
                size="sm"
                type="text"
                @update:model-value="
                  emit('update:targetEndpointId', $event)
                "
              />
              <StimText
                v-if="!session.live"
                as="span"
                size="caption"
                tone="tertiary"
              >
                Mock sessions are read-only.
              </StimText>
            </StimStack>
          </StimDisclosure>
          <span hidden data-probe="active-conversation-id">{{
            activeConversationId ?? ""
          }}</span>
          <span hidden data-probe="last-response-text">{{
            lastResponseText ?? ""
          }}</span>
          <span hidden data-probe="last-response-source">{{
            lastResponseSource ?? ""
          }}</span>
          <span hidden data-probe="last-final-sent-text">{{
            lastFinalSentText ?? ""
          }}</span>
        </StimStack>
      </StimPane>
    </StimStack>
  </StimPane>
</template>
