<script setup lang="ts">
import {
  StimBadge,
  StimButton,
  StimDisclosure,
  StimInfoList,
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
    props.session.live && !props.isLoading && props.draftText.trim().length > 0,
);
const latestToolActivity = computed(
  () => props.session.toolActivities.at(-1) ?? null,
);
const TOOL_OUTPUT_PRESSURE_LIMIT = 10_000;
const selectedParticipant = computed(
  () =>
    props.registeredParticipants.find(
      (participant) =>
        participant.participant_id === props.selectedParticipantId,
    ) ?? null,
);
const toolActivitySummary = computed(() => {
  const activity = latestToolActivity.value;
  if (!activity) {
    return null;
  }

  const result = activity.output_summary ?? activity.result_state;
  const pressure = toolOutputPressureLabel(
    activity.stdout_chars,
    activity.stderr_chars,
  );
  return [
    `${props.session.toolActivities.length} activity`,
    activity.tool_name,
    result,
    pressure,
  ]
    .filter(Boolean)
    .join(" · ");
});

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

function handleComposerEnter(event: KeyboardEvent) {
  if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }

  if (!canSend.value) {
    return;
  }

  event.preventDefault();
  emit("send");
}

function toolOutputPressureLabel(
  stdoutChars: number | null,
  stderrChars: number | null,
): string | null {
  const totalChars = (stdoutChars ?? 0) + (stderrChars ?? 0);
  if (totalChars < TOOL_OUTPUT_PRESSURE_LIMIT) {
    return null;
  }

  return "large tool output";
}
</script>

<template>
  <StimPane grow padding="none" radius="none" border="none">
    <StimPane border="subtle" grow padding="none" radius="none">
      <StimStack gap="none" grow full-block>
        <StimPane border="none" padding="md" radius="none">
          <StimPane
            border="none"
            inline-size="100%"
            max-inline-size="44rem"
            padding="none"
            radius="none"
          >
            <StimInline justify="between" align="start">
              <StimStack gap="xs">
                <StimInline gap="sm" wrap>
                  <StimText as="p" size="eyebrow" tone="secondary"
                    >messages</StimText
                  >
                  <StimBadge :tone="session.live ? 'accent' : 'muted'">
                    {{ session.live ? "live" : "mock" }}
                  </StimBadge>
                </StimInline>
                <StimText as="h2" size="label">{{ session.title }}</StimText>
                <StimText as="p" size="caption" tone="secondary">
                  {{
                    session.live
                      ? "Controller-backed desktop thread"
                      : "Static reference thread for layout pressure"
                  }}
                </StimText>
              </StimStack>
              <StimStack align="end" gap="xs">
                <StimText as="span" size="label">{{
                  selectedParticipant?.display_label ?? session.participantLabel
                }}</StimText>
                <StimText as="span" size="caption" tone="secondary">
                  {{ selectedParticipant?.status ?? controllerStatus }}
                </StimText>
              </StimStack>
            </StimInline>
          </StimPane>
        </StimPane>

        <StimPane
          ref="threadPaneRef"
          data-probe="landing-card"
          grow
          padding="md"
          radius="none"
          scroll
          tone="muted"
        >
          <StimStack data-probe="chat-thread" gap="md">
            <StimText
              v-if="session.messages.length === 0"
              as="p"
              size="body"
              tone="secondary"
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
          border="subtle"
          padding="lg"
          radius="none"
          tone="default"
        >
          <StimPane
            border="none"
            inline-size="100%"
            max-inline-size="44rem"
            padding="none"
            radius="none"
          >
            <StimStack gap="sm">
              <StimStack gap="xs">
                <StimText as="p" size="label">Composer</StimText>
                <StimText as="p" size="caption" tone="secondary">
                  Send a text roundtrip through the live controller thread.
                </StimText>
              </StimStack>
              <StimInput
                :model-value="draftText"
                data-probe="message-input"
                placeholder="Type a message"
                type="text"
                @keydown.enter="handleComposerEnter"
                @update:model-value="emit('update:draftText', $event)"
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
              <StimInline gap="sm" wrap>
                <StimButton
                  :disabled="!canSend"
                  :label="isLoading ? 'Sending…' : 'Send message'"
                  variant="primary"
                  @click="emit('send')"
                />
                <StimText
                  v-if="!session.live"
                  as="span"
                  size="caption"
                  tone="secondary"
                >
                  Mock sessions are read-only in this slice.
                </StimText>
              </StimInline>
              <StimDisclosure
                summary="Delivery settings"
                :caption="selectedParticipantId ?? targetEndpointId"
              >
                <StimStack gap="xs">
                  <StimText as="p" size="caption" tone="secondary">
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
                      @click="emit('selectParticipant', participant.participant_id)"
                    />
                  </StimInline>
                  <StimText
                    v-if="participantErrorMessage"
                    as="p"
                    size="caption"
                    tone="secondary"
                  >
                    {{ participantErrorMessage }}
                  </StimText>
                </StimStack>
                <StimInput
                  :model-value="targetEndpointId"
                  data-probe="target-endpoint-input"
                  :disabled="!session.live"
                  placeholder="target endpoint"
                  type="text"
                  @update:model-value="emit('update:targetEndpointId', $event)"
                />
              </StimDisclosure>
            </StimStack>
          </StimPane>
        </StimPane>

        <StimPane border="subtle" padding="lg" radius="none" tone="default">
          <StimPane
            border="none"
            inline-size="100%"
            max-inline-size="44rem"
            padding="none"
            radius="none"
          >
            <StimDisclosure
              summary="Controller diagnostics"
              :caption="controllerStatus"
            >
              <StimInfoList gap="sm">
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
                  <dd data-probe="active-conversation-id">
                    {{ activeConversationId }}
                  </dd>
                </div>
                <div v-if="lastResponseText">
                  <dt>Response</dt>
                  <dd data-probe="last-response-text">{{ lastResponseText }}</dd>
                </div>
                <div v-if="lastResponseSource">
                  <dt>Response source</dt>
                  <dd data-probe="last-response-source">
                    {{ lastResponseSource }}
                  </dd>
                </div>
                <div v-if="lastFinalSentText">
                  <dt>Final sent text</dt>
                  <dd data-probe="last-final-sent-text">
                    {{ lastFinalSentText }}
                  </dd>
                </div>
                <div v-if="toolActivitySummary">
                  <dt>Tool activity</dt>
                  <dd data-probe="tool-activity-summary">
                    {{ toolActivitySummary }}
                  </dd>
                </div>
                <div v-if="errorMessage">
                  <dt>Error</dt>
                  <dd>{{ errorMessage }}</dd>
                </div>
              </StimInfoList>
            </StimDisclosure>
          </StimPane>
        </StimPane>
      </StimStack>
    </StimPane>
  </StimPane>
</template>
