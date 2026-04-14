import "@stim-io/components/styles/foundation/index.css";
import "@stim-io/components/styles/themes/dark.css";
import "@stim-io/components/styles/components/stim-button/common.css";
import "./app.css";

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
