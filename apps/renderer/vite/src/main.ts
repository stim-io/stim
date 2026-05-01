import { createApp } from "vue";

import App from "./App.vue";
import { setupInspectionActions } from "./inspection/actions";
import { setupInspectionProbes } from "./inspection/probes";
import { buildStimServerDiscoveryUrl } from "./server/client-proof";
import { applyTheme } from "./styles/bootstrap";

function renderBootstrapError(error: unknown) {
  const target = document.querySelector("#app") ?? document.body;
  const message =
    error instanceof Error ? (error.stack ?? error.message) : String(error);

  const pre = document.createElement("pre");
  pre.textContent = `bootstrap failed\n\n${message}`;
  target.replaceChildren(pre);
}

async function bootstrap() {
  await applyTheme();
  createApp(App).mount("#app");
  void setupInspectionActions();
  void setupInspectionProbes();
  buildStimServerDiscoveryUrl("bootstrap-proof");
}

void bootstrap().catch((error) => {
  renderBootstrapError(error);
});
