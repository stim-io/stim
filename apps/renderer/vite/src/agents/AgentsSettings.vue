<script setup lang="ts">
import {
  StimBadge,
  StimButton,
  StimInfoList,
  StimInline,
  StimPane,
  StimStack,
  StimText,
} from "@stim-io/components";
import { useAgentsSettings } from "./useAgentsSettings";

const {
  activeInstanceId,
  agentsRuntime,
  applyingProfileKey,
  applyProfile,
  enabledCapabilityLabels,
  errorMessage,
  instances,
  isLoading,
  launchInstance,
  makeActive,
  managingInstanceId,
  probeInstance,
  probeProvider,
  probingInstanceId,
  probingProviderInstanceId,
  profileApplyKey,
  profiles,
  refreshAgents,
  registeredAgents,
  registeredParticipants,
  registryErrorMessage,
  runtimeBaseUrl,
  runtimeState,
  selectParticipant,
  selectedParticipantId,
  selectingInstanceId,
  selectingParticipantId,
  stopInstance,
} = useAgentsSettings();
</script>

<template>
  <StimPane grow padding="none" radius="none" border="none">
    <StimStack gap="none" grow full-block>
      <StimPane border="subtle" padding="md" radius="none">
        <StimInline justify="between" align="start">
          <StimStack gap="xs">
            <StimInline gap="sm" wrap>
              <StimText as="p" size="eyebrow" tone="secondary">agents</StimText>
              <StimBadge :tone="runtimeState === 'ready' ? 'accent' : 'muted'">
                {{ runtimeState }}
              </StimBadge>
            </StimInline>
            <StimText as="h2" size="label">Agents settings</StimText>
            <StimText as="p" size="caption" tone="secondary">
              Santi instance control surface exposed by the local agents
              sidecar.
            </StimText>
          </StimStack>
          <StimButton
            :disabled="isLoading"
            :label="isLoading ? 'Refreshing...' : 'Refresh'"
            variant="secondary"
            @click="refreshAgents"
          />
        </StimInline>
      </StimPane>

      <StimPane grow padding="md" radius="none" scroll tone="muted">
        <StimStack gap="md">
          <StimPane border="subtle" padding="md" radius="sm" tone="default">
            <StimStack gap="sm">
              <StimText as="h3" size="label">Agents runtime</StimText>
              <StimInfoList gap="sm">
                <div>
                  <dt>Namespace</dt>
                  <dd>{{ agentsRuntime?.namespace ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Endpoint</dt>
                  <dd>{{ runtimeBaseUrl }}</dd>
                </div>
                <div>
                  <dt>Instance</dt>
                  <dd>{{ agentsRuntime?.instance_id ?? "unknown" }}</dd>
                </div>
                <div v-if="agentsRuntime?.detail">
                  <dt>Detail</dt>
                  <dd>{{ agentsRuntime.detail }}</dd>
                </div>
              </StimInfoList>
            </StimStack>
          </StimPane>

          <StimPane border="subtle" padding="md" radius="sm" tone="default">
            <StimStack gap="sm">
              <StimInline justify="between" align="center">
                <StimText as="h3" size="label">Agent profiles</StimText>
                <StimBadge tone="muted">
                  {{ profiles.length }}
                </StimBadge>
              </StimInline>
              <StimInfoList gap="sm">
                <div
                  v-for="profile in profiles"
                  :key="profile.id"
                  data-probe="agent-profile-row"
                >
                  <dt>{{ profile.label }}</dt>
                  <dd>
                    {{ profile.id }} · {{ profile.launch_profile }} ·
                    {{ profile.provider.api }} · {{ profile.provider.model }} ·
                    {{ profile.secret_state }}
                  </dd>
                </div>
              </StimInfoList>
            </StimStack>
          </StimPane>

          <StimText
            v-if="errorMessage"
            as="p"
            data-probe="agents-error-message"
            size="caption"
            tone="danger"
          >
            {{ errorMessage }}
          </StimText>

          <StimPane border="subtle" padding="md" radius="sm" tone="default">
            <StimStack gap="sm">
              <StimInline justify="between" align="center">
                <StimText as="h3" size="label">Registered agents</StimText>
                <StimBadge tone="muted">
                  {{ registeredAgents.length }}
                </StimBadge>
              </StimInline>
              <StimText
                v-if="registryErrorMessage"
                as="p"
                size="caption"
                tone="secondary"
              >
                {{ registryErrorMessage }}
              </StimText>
              <StimInfoList v-else gap="sm">
                <div
                  v-for="agent in registeredAgents"
                  :key="agent.instance_id"
                  data-probe="registered-agent-row"
                >
                  <dt>{{ agent.label }}</dt>
                  <dd>
                    {{ agent.agent_id }} · {{ agent.instance_id }} ·
                    {{ agent.participant_id }} ·
                    {{ agent.delivery_endpoint_id ?? "no-delivery-target" }} ·
                    {{ agent.status }} ·
                    {{ agent.last_event_id }}
                  </dd>
                </div>
              </StimInfoList>
            </StimStack>
          </StimPane>

          <StimPane border="subtle" padding="md" radius="sm" tone="default">
            <StimStack gap="sm">
              <StimInline justify="between" align="center">
                <StimText as="h3" size="label">Chat participants</StimText>
                <StimBadge tone="muted">
                  {{ registeredParticipants.length }}
                </StimBadge>
              </StimInline>
              <StimInfoList gap="sm">
                <div
                  v-for="participant in registeredParticipants"
                  :key="participant.participant_id"
                  data-probe="registered-participant-row"
                >
                  <dt>{{ participant.display_label }}</dt>
                  <dd>
                    {{ participant.participant_id }} ·
                    {{ participant.status }} ·
                    {{
                      participant.delivery_target?.endpoint_id ??
                      "no-delivery-target"
                    }}
                    ·
                    {{ participant.last_event_id }}
                  </dd>
                </div>
              </StimInfoList>
              <StimInline gap="sm" wrap>
                <StimButton
                  v-for="participant in registeredParticipants"
                  :key="participant.participant_id"
                  :disabled="
                    selectingParticipantId === participant.participant_id ||
                    selectedParticipantId === participant.participant_id
                  "
                  :label="
                    selectingParticipantId === participant.participant_id
                      ? 'Selecting...'
                      : participant.display_label
                  "
                  :pressed="
                    selectedParticipantId === participant.participant_id
                  "
                  data-probe="chat-participant-select-button"
                  size="sm"
                  variant="ghost"
                  @click="selectParticipant(participant.participant_id)"
                />
              </StimInline>
            </StimStack>
          </StimPane>

          <StimPane
            v-for="instance in instances"
            :key="instance.id"
            border="subtle"
            padding="md"
            radius="sm"
            tone="default"
            data-probe="agent-instance-card"
          >
            <StimStack gap="sm">
              <StimInline justify="between" align="start">
                <StimStack gap="xs">
                  <StimInline gap="sm" wrap>
                    <StimText as="h3" size="label">{{
                      instance.label
                    }}</StimText>
                    <StimBadge
                      :tone="instance.state === 'ready' ? 'accent' : 'muted'"
                    >
                      {{ instance.state }}
                    </StimBadge>
                    <StimBadge tone="muted">
                      {{ instance.managed ? "managed" : "attached" }}
                    </StimBadge>
                    <StimBadge v-if="instance.active" tone="accent">
                      active
                    </StimBadge>
                  </StimInline>
                  <StimText as="p" size="caption" tone="secondary">
                    {{ instance.detail ?? "No probe detail available." }}
                  </StimText>
                </StimStack>
                <StimInline gap="sm" align="center">
                  <StimText as="span" size="caption" tone="secondary">
                    {{ instance.agent_kind }}
                  </StimText>
                  <StimButton
                    :disabled="
                      selectingInstanceId === instance.id ||
                      activeInstanceId === instance.id
                    "
                    :label="
                      selectingInstanceId === instance.id
                        ? 'Selecting...'
                        : 'Use'
                    "
                    :pressed="activeInstanceId === instance.id"
                    data-probe="agent-instance-select-button"
                    size="sm"
                    variant="ghost"
                    @click="makeActive(instance.id)"
                  />
                  <StimButton
                    :disabled="probingInstanceId === instance.id"
                    :label="
                      probingInstanceId === instance.id ? 'Probing...' : 'Probe'
                    "
                    data-probe="agent-instance-probe-button"
                    size="sm"
                    variant="ghost"
                    @click="probeInstance(instance.id)"
                  />
                  <StimButton
                    :disabled="probingProviderInstanceId === instance.id"
                    :label="
                      probingProviderInstanceId === instance.id
                        ? 'Checking...'
                        : 'Provider'
                    "
                    data-probe="agent-provider-probe-button"
                    size="sm"
                    variant="ghost"
                    @click="probeProvider(instance.id)"
                  />
                  <StimButton
                    v-if="instance.managed"
                    :disabled="
                      managingInstanceId === instance.id ||
                      instance.process !== null
                    "
                    :label="
                      managingInstanceId === instance.id &&
                      instance.process === null
                        ? 'Launching...'
                        : 'Launch'
                    "
                    data-probe="agent-instance-launch-button"
                    size="sm"
                    variant="secondary"
                    @click="launchInstance(instance.id)"
                  />
                  <StimButton
                    v-if="instance.managed"
                    :disabled="
                      managingInstanceId === instance.id ||
                      instance.process === null
                    "
                    :label="
                      managingInstanceId === instance.id &&
                      instance.process !== null
                        ? 'Stopping...'
                        : 'Stop'
                    "
                    data-probe="agent-instance-stop-button"
                    size="sm"
                    variant="ghost"
                    @click="stopInstance(instance.id)"
                  />
                </StimInline>
              </StimInline>

              <StimInline gap="sm" wrap>
                <StimButton
                  v-for="profile in profiles"
                  :key="`${instance.id}:${profile.id}`"
                  :disabled="
                    profile.secret_state !== 'available' ||
                    applyingProfileKey ===
                      profileApplyKey(instance.id, profile.id)
                  "
                  :label="
                    applyingProfileKey ===
                    profileApplyKey(instance.id, profile.id)
                      ? 'Applying...'
                      : `Apply ${profile.label}`
                  "
                  data-probe="agent-profile-apply-button"
                  size="sm"
                  variant="secondary"
                  @click="applyProfile(instance.id, profile.id)"
                />
              </StimInline>

              <StimInfoList gap="sm">
                <div>
                  <dt>Agent</dt>
                  <dd>{{ instance.agent_id }}</dd>
                </div>
                <div>
                  <dt>Participant</dt>
                  <dd>{{ instance.participant_id }}</dd>
                </div>
                <div>
                  <dt>Delivery endpoint</dt>
                  <dd>{{ instance.delivery_endpoint_id }}</dd>
                </div>
                <div>
                  <dt>Endpoint</dt>
                  <dd>{{ instance.endpoint ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Profile</dt>
                  <dd>{{ instance.profile ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Process</dt>
                  <dd>
                    {{
                      instance.process
                        ? `pid ${instance.process.pid}`
                        : "not launched by agents"
                    }}
                  </dd>
                </div>
                <div>
                  <dt>Santi mode</dt>
                  <dd>{{ instance.service?.mode ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Launch profile</dt>
                  <dd>{{ instance.service?.launch_profile ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Config version</dt>
                  <dd>
                    {{
                      instance.config
                        ? `${instance.config.source} #${instance.config.config_version}`
                        : "unknown"
                    }}
                  </dd>
                </div>
                <div>
                  <dt>Config event</dt>
                  <dd>{{ instance.config?.last_event_id ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Service</dt>
                  <dd>
                    {{ instance.service?.service_name ?? "unknown" }}
                    {{ instance.service?.service_version ?? "" }}
                  </dd>
                </div>
                <div>
                  <dt>Santi API</dt>
                  <dd>{{ instance.service?.api_version ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Capabilities</dt>
                  <dd>
                    {{
                      enabledCapabilityLabels(instance.service?.capabilities)
                    }}
                  </dd>
                </div>
                <div>
                  <dt>Provider API</dt>
                  <dd>{{ instance.provider?.api ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Model</dt>
                  <dd>{{ instance.provider?.model ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Gateway</dt>
                  <dd>
                    {{ instance.provider?.gateway_base_url ?? "unknown" }}
                  </dd>
                </div>
                <div>
                  <dt>Provider probe</dt>
                  <dd>{{ instance.provider_probe?.state ?? "unknown" }}</dd>
                </div>
                <div>
                  <dt>Probe URL</dt>
                  <dd>
                    {{ instance.provider_probe?.checked_url ?? "unknown" }}
                  </dd>
                </div>
                <div>
                  <dt>Probe HTTP</dt>
                  <dd>
                    {{ instance.provider_probe?.http_status ?? "unknown" }}
                  </dd>
                </div>
                <div>
                  <dt>Runtime root</dt>
                  <dd>{{ instance.runtime?.runtime_root ?? "unknown" }}</dd>
                </div>
              </StimInfoList>
            </StimStack>
          </StimPane>
        </StimStack>
      </StimPane>
    </StimStack>
  </StimPane>
</template>
