<script setup lang="ts">
import {
  StimAvatar,
  StimBadge,
  StimButton,
  StimInline,
  StimInput,
  StimInteractiveRow,
  StimPane,
  StimStack,
  StimText,
} from "@stim-io/components";

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
    border="subtle"
    :inline-size="collapsed ? '5.5rem' : '21rem'"
    :min-inline-size="collapsed ? '5.5rem' : '18rem'"
    padding="md"
    radius="none"
  >
      <StimStack gap="md">
        <StimInline justify="between">
        <StimStack gap="xs">
          <StimText as="p" size="eyebrow" tone="secondary">stim</StimText>
          <StimText v-if="!collapsed" as="h1" data-probe="landing-title" size="label">
            Desktop messaging
          </StimText>
        </StimStack>
        <StimInline gap="sm">
          <StimBadge
            v-if="!collapsed"
            :tone="controllerStatus === 'ready' ? 'accent' : 'muted'"
          >
            {{ controllerStatus }}
          </StimBadge>
          <StimButton
            :label="collapsed ? '→' : '←'"
            data-probe="session-drawer-toggle"
            @click="emit('toggleCollapse')"
          />
        </StimInline>
        </StimInline>

      <StimButton
        :label="collapsed ? '+' : 'New conversation'"
        data-probe="new-conversation-button"
        @click="emit('newConversation')"
      />

      <StimStack v-if="!collapsed" gap="sm">
        <StimInput
          :model-value="sessionQuery"
          data-probe="session-search-input"
          placeholder="Search sessions"
          @update:model-value="emit('update:sessionQuery', $event)"
        />

        <StimInline gap="sm" wrap>
          <StimButton
            :label="activeScope === 'all' ? 'All · active' : 'All'"
            data-probe="session-filter-all"
            @click="emit('update:activeScope', 'all')"
          />
          <StimButton
            :label="activeScope === 'live' ? 'Live · active' : 'Live'"
            data-probe="session-filter-live"
            @click="emit('update:activeScope', 'live')"
          />
          <StimButton
            :label="activeScope === 'unread' ? 'Unread · active' : 'Unread'"
            data-probe="session-filter-unread"
            @click="emit('update:activeScope', 'unread')"
          />
        </StimInline>
      </StimStack>

      <StimStack data-probe="session-list" gap="xs">
        <StimText
          v-if="sessions.length === 0"
          as="p"
          data-probe="session-list-empty"
          size="caption"
          tone="secondary"
        >
          No sessions match the current filter.
        </StimText>
        <StimInteractiveRow
          v-for="session in sessions"
          :key="session.id"
          :active="session.id === activeSessionId"
          :data-probe="session.id === activeSessionId ? 'active-session-item' : undefined"
          :data-session-id="session.id"
          tone="accent"
          @click="emit('select', session.id)"
        >
          <StimAvatar :label="session.participantLabel" tone="muted" size="md" />

          <StimStack
            v-if="!collapsed"
            gap="xs"
            style="flex: 1 1 auto; min-inline-size: 0"
          >
            <StimInline justify="between" gap="sm">
              <StimText as="span" size="label">{{ session.title }}</StimText>
              <StimText as="span" size="caption" tone="secondary">
                {{ session.activityLabel }}
              </StimText>
            </StimInline>
            <StimText as="p" size="caption" tone="secondary">
              {{ session.preview }}
            </StimText>
          </StimStack>

          <StimBadge v-if="session.unreadCount > 0" tone="accent">
            {{ session.unreadCount }}
          </StimBadge>
        </StimInteractiveRow>
      </StimStack>
    </StimStack>
  </StimPane>
</template>
