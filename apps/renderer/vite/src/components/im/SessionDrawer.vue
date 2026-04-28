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
    :inline-size="collapsed ? '4.25rem' : '18rem'"
    :min-inline-size="collapsed ? '4.25rem' : '16rem'"
    padding="sm"
    radius="none"
  >
    <StimStack v-if="collapsed" align="center" gap="sm">
      <StimButton
        aria-label="Expand session drawer"
        label="›"
        data-probe="session-drawer-toggle"
        size="sm"
        variant="ghost"
        @click="emit('toggleCollapse')"
      />

      <StimButton
        label="+"
        aria-label="New conversation"
        data-probe="new-conversation-button"
        size="sm"
        variant="secondary"
        @click="emit('newConversation')"
      />

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

    <StimStack v-else gap="sm">
      <StimInline justify="between">
        <StimStack gap="none">
          <StimText as="p" size="eyebrow" tone="secondary">stim</StimText>
          <StimText
            v-if="!collapsed"
            as="h1"
            data-probe="landing-title"
            size="label"
          >
            Desktop messaging
          </StimText>
        </StimStack>
        <StimInline gap="sm">
          <StimBadge
            v-if="!collapsed"
            size="sm"
            :tone="controllerStatus === 'ready' ? 'accent' : 'muted'"
          >
            {{ controllerStatus }}
          </StimBadge>
          <StimButton
            :aria-label="
              collapsed ? 'Expand session drawer' : 'Collapse session drawer'
            "
            :label="collapsed ? '›' : '‹'"
            data-probe="session-drawer-toggle"
            size="sm"
            variant="ghost"
            @click="emit('toggleCollapse')"
          />
        </StimInline>
      </StimInline>

      <StimButton
        label="New conversation"
        aria-label="New conversation"
        data-probe="new-conversation-button"
        size="sm"
        variant="secondary"
        @click="emit('newConversation')"
      />

      <StimStack gap="sm">
        <StimInput
          :model-value="sessionQuery"
          data-probe="session-search-input"
          placeholder="Search sessions"
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
          :aria-label="session.title"
          :data-probe="
            session.id === activeSessionId ? 'active-session-item' : undefined
          "
          :data-session-id="session.id"
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

          <StimStack grow gap="none">
            <StimInline justify="between" gap="sm">
              <StimText as="span" size="label" truncate>
                {{ session.title }}
              </StimText>
              <StimText as="span" size="caption" tone="secondary" truncate>
                {{ session.activityLabel }}
              </StimText>
            </StimInline>
            <StimText as="p" size="caption" tone="secondary" truncate>
              {{ session.preview }}
            </StimText>
          </StimStack>

          <StimBadge v-if="session.unreadCount > 0" tone="accent" size="sm">
            {{ session.unreadCount }}
          </StimBadge>
        </StimInteractiveRow>
      </StimStack>
    </StimStack>
  </StimPane>
</template>
