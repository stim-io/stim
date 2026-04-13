import { createApp } from "vue";

import App from "./App.vue";
import { setupInspectionProbes } from "./inspection/probes";
import { applyTheme } from "./styles/bootstrap";

async function bootstrap() {
    await applyTheme();
    await setupInspectionProbes();
    createApp(App).mount("#app");
}

void bootstrap();
