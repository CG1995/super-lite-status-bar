import { createApi } from "./components/state.js";
import { renderFloatingBar } from "./floating_bar/floating.js";
import { renderSettings } from "./settings/settings.js";
import { renderTooltip } from "./tray/tooltip.js";

const route = window.location.hash.replace("#", "") || "settings";
const app = document.getElementById("app");
const api = createApi();

async function bootstrap() {
  const [config, metrics] = await Promise.all([api.getConfig(), api.getMetrics()]);
  applyTheme(config);

  if (route === "floating") {
    renderFloatingBar(app, api, config, metrics);
  } else if (route === "tooltip") {
    renderTooltip(app, api, config, metrics);
  } else {
    await renderSettings(app, api, config, metrics);
  }

  await api.listen("config-updated", (nextConfig) => {
    applyTheme(nextConfig);
    window.dispatchEvent(new CustomEvent("app-config", { detail: nextConfig }));
  });

  await api.listen("metrics-updated", (snapshot) => {
    window.dispatchEvent(new CustomEvent("app-metrics", { detail: snapshot }));
  });
}

function applyTheme(config) {
  document.documentElement.dataset.theme = config.theme || "system";
  document.documentElement.style.setProperty("--status-font-size", `${effectiveFontSize(config)}px`);
}

function effectiveFontSize(config) {
  const preset = config.font?.preset || "small";
  if (preset === "large") return 16;
  if (preset === "custom") return Math.min(28, Math.max(12, Number(config.font?.custom_px || 12)));
  if (preset === "medium") return 14;
  return 12;
}

bootstrap().catch((error) => {
  app.innerHTML = `<section class="fatal">启动失败：${String(error)}</section>`;
});
