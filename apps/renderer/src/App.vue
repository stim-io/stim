<script setup lang="ts">
import { StimButton } from "@stim-io/components";
</script>

<template>
  <main class="landing-shell" data-probe="landing-shell">
    <section class="landing-card" data-probe="landing-card">
      <p class="landing-eyebrow">stim</p>
      <h1 class="landing-title" data-probe="landing-title">
        Agent-native messaging, starting with a strict desktop landing.
      </h1>
      <p class="landing-copy">
        This first slice proves the renderer, Tauri shell, and shared
        component/theme boundary without letting host or runtime concerns leak
        into the UI surface.
      </p>
      <div class="landing-actions" data-probe="landing-actions">
        <StimButton label="Open the loop" />
      </div>
    </section>
  </main>
</template>
