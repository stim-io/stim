<script setup lang="ts">
import {
  StimAvatar,
  StimBadge,
  StimButton,
  StimConversationRow,
  StimInline,
  StimInput,
  StimInteractiveRow,
  StimPane,
  StimStack,
  StimText,
} from "@stim-io/components";
import {
  IconChevronLeft,
  IconChevronRight,
  IconNewConversation,
} from "@stim-io/icons";

import type { SessionSummary } from "./types";

defineProps<{
  sessions: SessionSummary[];
  activeSessionId: string;
  controllerStatus: string;
  collapsed: boolean;
  sessionQuery: string;
  activeScope: "all" | "live" | "unread";
}>();

const emit = defineEmits<{
  select: [sessionId: string];
  newConversation: [];
  toggleCollapse: [];
  "update:sessionQuery": [value: string];
  "update:activeScope": [value: "all" | "live" | "unread"];
}>();
</script>

<template>
  <StimPane
    class="session-drawer"
    :data-collapsed="collapsed ? 'true' : 'false'"
    data-probe="session-drawer"
    border="none"
    :inline-size="collapsed ? '4.25rem' : '17rem'"
    :min-inline-size="collapsed ? '4.25rem' : '15rem'"
    padding="sm"
    radius="none"
    tone="muted"
  >
    <StimStack v-if="collapsed" align="center" gap="sm">
      <StimButton
        aria-label="Expand session drawer"
        data-probe="session-drawer-toggle"
        size="sm"
        variant="ghost"
        @click="emit('toggleCollapse')"
      >
        <IconChevronRight size="sm" />
      </StimButton>

      <StimButton
        aria-label="New conversation"
        data-probe="new-conversation-button"
        size="sm"
        variant="ghost"
        @click="emit('newConversation')"
      >
        <IconNewConversation size="sm" />
      </StimButton>

      <StimStack data-probe="session-list" gap="xs">
        <StimInteractiveRow
          v-for="session in sessions"
          :key="session.id"
          :active="session.id === activeSessionId"
          :aria-label="session.title"
          :data-probe="
            session.id === activeSessionId ? 'active-session-item' : undefined
          "
          :data-session-id="session.id"
          justify="center"
          padding="sm"
          radius="sm"
          tone="accent"
          @click="emit('select', session.id)"
        >
          <StimAvatar
            :label="session.participantLabel"
            tone="muted"
            size="sm"
          />
        </StimInteractiveRow>
      </StimStack>
    </StimStack>

    <StimStack v-else gap="md">
      <StimInline justify="between" align="center">
        <StimText
          as="span"
          data-probe="landing-title"
          size="body"
          tone="primary"
          weight="semibold"
        >
          Messages
        </StimText>
        <StimButton
          aria-label="Collapse session drawer"
          data-probe="session-drawer-toggle"
          size="sm"
          variant="ghost"
          @click="emit('toggleCollapse')"
        >
          <IconChevronLeft size="sm" />
        </StimButton>
      </StimInline>

      <StimInput
        :model-value="sessionQuery"
        data-probe="session-search-input"
        placeholder="Search"
        size="sm"
        @update:model-value="emit('update:sessionQuery', $event)"
      />

      <StimInline gap="xs" wrap>
        <StimButton
          label="All"
          :pressed="activeScope === 'all'"
          data-probe="session-filter-all"
          size="sm"
          :variant="activeScope === 'all' ? 'secondary' : 'ghost'"
          @click="emit('update:activeScope', 'all')"
        />
        <StimButton
          label="Live"
          :pressed="activeScope === 'live'"
          data-probe="session-filter-live"
          size="sm"
          :variant="activeScope === 'live' ? 'secondary' : 'ghost'"
          @click="emit('update:activeScope', 'live')"
        />
        <StimButton
          label="Unread"
          :pressed="activeScope === 'unread'"
          data-probe="session-filter-unread"
          size="sm"
          :variant="activeScope === 'unread' ? 'secondary' : 'ghost'"
          @click="emit('update:activeScope', 'unread')"
        />
      </StimInline>

      <StimInline justify="between" align="center">
        <StimButton
          aria-label="New conversation"
          data-probe="new-conversation-button"
          size="sm"
          variant="ghost"
          @click="emit('newConversation')"
        >
          <IconNewConversation size="sm" />
          New conversation
        </StimButton>
        <StimBadge size="sm" tone="muted">
          {{ controllerStatus }}
        </StimBadge>
      </StimInline>

      <StimStack data-probe="session-list" gap="none">
        <StimText
          v-if="sessions.length === 0"
          as="p"
          data-probe="session-list-empty"
          size="caption"
          tone="tertiary"
        >
          No sessions match the current filter.
        </StimText>
        <StimConversationRow
          v-for="session in sessions"
          :key="session.id"
          :name="session.title"
          :preview="session.preview"
          :time-label="session.activityLabel"
          :unread-count="session.unreadCount"
          :selected="session.id === activeSessionId"
          :avatar-label="session.participantLabel"
          :avatar-tone="session.id === activeSessionId ? 'accent' : 'default'"
          :data-probe="
            session.id === activeSessionId ? 'active-session-item' : undefined
          "
          :data-session-id="session.id"
          :aria-label="session.title"
          @click="emit('select', session.id)"
        />
      </StimStack>
    </StimStack>
  </StimPane>
</template>
