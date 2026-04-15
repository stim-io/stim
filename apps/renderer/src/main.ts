import { createApp } from "vue";

import App from "./App.vue";
import { setupInspectionProbes } from "./inspection/probes";
import { buildStimServerDiscoveryUrl } from "./server/client-proof";
import { applyTheme } from "./styles/bootstrap";

function renderBootstrapError(error: unknown) {
  const target = document.querySelector("#app") ?? document.body;
  const message =
    error instanceof Error ? (error.stack ?? error.message) : String(error);

  target.innerHTML = `<pre style="padding:16px;white-space:pre-wrap;color:#b91c1c;">bootstrap failed\n\n${message}</pre>`;
}

async function bootstrap() {
  await applyTheme();
  createApp(App).mount("#app");
  void setupInspectionProbes();
  buildStimServerDiscoveryUrl("bootstrap-proof");
}

void bootstrap().catch((error) => {
  renderBootstrapError(error);
});
