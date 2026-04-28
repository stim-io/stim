import "@stim-io/components/styles/foundation/index.css";
import "@stim-io/components/styles/themes/dark.css";
import "@stim-io/components/styles/components/stim-button/common.css";
import "@stim-io/components/styles/components/stim-disclosure/common.css";
import "@stim-io/components/styles/components/stim-input/common.css";
import "@stim-io/components/styles/components/stim-text/common.css";
import "@stim-io/components/styles/components/stim-app-root/common.css";
import "@stim-io/components/styles/components/stim-avatar/common.css";
import "@stim-io/components/styles/components/stim-badge/common.css";
import "@stim-io/components/styles/components/stim-info-list/common.css";
import "@stim-io/components/styles/components/stim-inline/common.css";
import "@stim-io/components/styles/components/stim-interactive-row/common.css";
import "@stim-io/components/styles/components/stim-pane/common.css";
import "@stim-io/components/styles/components/stim-surface/common.css";
import "@stim-io/components/styles/components/stim-split/common.css";
import "@stim-io/components/styles/components/stim-stack/common.css";
import "@stim-io/components/styles/components/stim-viewport-stage/common.css";
import "@stim-io/components/styles/components/stim-message-card-frame/common.css";
import "@stim-io/components/styles/components/stim-message-row/common.css";
import "@stim-io/components/styles/components/stim-rich-content/common.css";

function resolveEngineThemePatch() {
  const userAgent = navigator.userAgent.toLowerCase();
  const isWebKit =
    /safari/.test(userAgent) && !/chrome|chromium|edg/.test(userAgent);

  return isWebKit
    ? import("@stim-io/components/styles/components/stim-button/webkit.css")
    : import("@stim-io/components/styles/components/stim-button/chromium.css");
}

export async function applyTheme() {
  document.documentElement.dataset.theme = "dark";
  await resolveEngineThemePatch();
}
