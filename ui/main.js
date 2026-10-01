import { createApi } from "./components/state.js";
import { renderFloatingBar } from "./floating_bar/floating.js";
import { renderSettings } from "./settings/settings.js";
import { renderTooltip } from "./tray/tooltip.js";

const route = window.location.hash.replace("#", "") || "settings";
const app = document.getElementById("app");
const api = createApi();

const FONT_PX = { small: 12, medium: 13, large: 15 };

async function bootstrap() {
  document.documentElement.dataset.surface = route;
  const [config, metrics] = await Promise.all([api.getConfig(), api.getMetrics()]);
  applyTheme(config);

  // Listen before rendering so no update between the initial fetch and render is lost.
  await api.listen("config-updated", (nextConfig) => {
    applyTheme(nextConfig);
    window.dispatchEvent(new CustomEvent("app-config", { detail: nextConfig }));
  });
  await api.listen("metrics-updated", (snapshot) => {
    window.dispatchEvent(new CustomEvent("app-metrics", { detail: snapshot }));
  });
  if (!api.isTauri) {
    window.addEventListener("app-config", (event) => applyTheme(event.detail));
  }

  if (route === "floating") {
    renderFloatingBar(app, api, config, metrics);
  } else if (route === "tooltip") {
    renderTooltip(app, api, config, metrics);
  } else {
    await renderSettings(app, api, config, metrics);
  }
}

function applyTheme(config) {
  document.documentElement.dataset.theme = config.theme || "system";
  const px = FONT_PX[config.font?.preset] || FONT_PX.small;
  document.documentElement.style.setProperty("--status-font-size", `${px}px`);
}

bootstrap().catch((error) => {
  app.innerHTML = `<section class="fatal">启动失败：${String(error)}</section>`;
});
