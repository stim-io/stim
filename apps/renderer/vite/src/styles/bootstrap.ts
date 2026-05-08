import "@stim-io/components/styles/foundation/index.css";
import "@stim-io/components/styles/themes/light.css";
import "@stim-io/icons/styles/index.css";
import "@stim-io/components/styles/components/atoms/stim-button/common.css";
import "@stim-io/components/styles/components/atoms/stim-input/common.css";
import "@stim-io/components/styles/components/atoms/stim-text/common.css";
import "@stim-io/components/styles/components/atoms/stim-avatar/common.css";
import "@stim-io/components/styles/components/atoms/stim-badge/common.css";
import "@stim-io/components/styles/components/primitives/stim-disclosure/common.css";
import "@stim-io/components/styles/components/primitives/stim-app-root/common.css";
import "@stim-io/components/styles/components/primitives/stim-info-list/common.css";
import "@stim-io/components/styles/components/primitives/stim-inline/common.css";
import "@stim-io/components/styles/components/primitives/stim-interactive-row/common.css";
import "@stim-io/components/styles/components/primitives/stim-pane/common.css";
import "@stim-io/components/styles/components/primitives/stim-surface/common.css";
import "@stim-io/components/styles/components/primitives/stim-split/common.css";
import "@stim-io/components/styles/components/primitives/stim-stack/common.css";
import "@stim-io/components/styles/components/primitives/stim-viewport-stage/common.css";
import "@stim-io/components/styles/components/composition/stim-message-card-frame/common.css";
import "@stim-io/components/styles/components/composition/stim-message-row/common.css";
import "@stim-io/components/styles/components/composition/stim-rich-content/common.css";
import "@stim-io/components/styles/components/composition/stim-conversation-row/common.css";
import "@stim-io/components/styles/components/composition/stim-composer/common.css";

function resolveEngineThemePatch() {
  const userAgent = navigator.userAgent.toLowerCase();
  const isWebKit =
    /safari/.test(userAgent) && !/chrome|chromium|edg/.test(userAgent);

  return isWebKit
    ? import("@stim-io/components/styles/components/atoms/stim-button/webkit.css")
    : import("@stim-io/components/styles/components/atoms/stim-button/chromium.css");
}

export async function applyTheme() {
  document.documentElement.dataset.theme = "light";
  await resolveEngineThemePatch();
}
