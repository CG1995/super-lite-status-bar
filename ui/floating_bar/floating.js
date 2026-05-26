import { pressureClass, shortSpeed } from "../components/format.js";

export function renderFloatingBar(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  root.className = "floating-shell";
  root.innerHTML = `<section class="floating" data-floating></section>`;

  const bar = root.querySelector("[data-floating]");
  const render = (snapshot) => {
    if (!snapshot) return;
    currentMetrics = snapshot;
    bar.className = `floating ${pressureClass(snapshot)}`;
    bar.style.opacity = String(config.floating_bar.opacity ?? 0.92);
    bar.innerHTML = floatingContent(snapshot);
  };

  render(initialMetrics);
  window.addEventListener("app-metrics", (event) => render(event.detail));
  window.addEventListener("app-config", (event) => {
    config = event.detail;
    render(currentMetrics);
  });

  bar.addEventListener("mousedown", (event) => {
    if (!config.floating_bar.lock_position && event.button === 0) {
      api.startDragging();
    }
  });
}

function floatingContent(snapshot) {
  return `
    ${item("CPU", `${Math.round(snapshot.cpu_percent)}%`)}
    ${item("MEM", `${Math.round(snapshot.memory.percent)}%`)}
    ${item("GPU", gpuValue(snapshot))}
    ${item("↓", shortSpeed(snapshot.network.download_bps, "auto"))}
    ${item("↑", shortSpeed(snapshot.network.upload_bps, "auto"))}
  `;
}

function item(label, value) {
  return `<span><b>${label}</b> ${value}</span>`;
}

function gpuValue(snapshot) {
  if (snapshot.gpu?.usage_percent != null) return `${Math.round(snapshot.gpu.usage_percent)}%`;
  return "N/A";
}
