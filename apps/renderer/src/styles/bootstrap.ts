import "@stim-io/stim-components/style.css";
import "@stim-io/stim-components/tokens.css";
import "@stim-io/stim-components/themes/dark/common.css";
import "./app.css";

function resolveEngineThemePatch() {
  const userAgent = navigator.userAgent.toLowerCase();
  const isWebKit =
    /safari/.test(userAgent) && !/chrome|chromium|edg/.test(userAgent);

  return isWebKit
    ? import("@stim-io/stim-components/themes/dark/webkit.css")
    : import("@stim-io/stim-components/themes/dark/chromium.css");
}

export async function applyTheme() {
  document.documentElement.dataset.theme = "dark";
  await resolveEngineThemePatch();
}
