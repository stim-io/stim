import { createApp } from "vue";

import App from "./App.vue";
import { setupInspectionProbes } from "./inspection/probes";
import { buildStimServerDiscoveryUrl } from "./server/client-proof";
import { applyTheme } from "./styles/bootstrap";

async function bootstrap() {
  await applyTheme();
  await setupInspectionProbes();
  buildStimServerDiscoveryUrl("bootstrap-proof");
  createApp(App).mount("#app");
}

void bootstrap();
