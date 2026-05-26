import { bytes, percent, pressureClass, shortSpeed } from "../components/format.js";

export function renderTooltip(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  root.className = "tooltip-shell";
  root.innerHTML = `
    <section class="tray-tooltip ${pressureClass(initialMetrics)}" data-tooltip></section>
  `;

  const tooltip = root.querySelector("[data-tooltip]");
  const render = (snapshot) => {
    if (!snapshot) return;
    currentMetrics = snapshot;
    tooltip.className = `tray-tooltip ${pressureClass(snapshot)}`;
    tooltip.innerHTML = tooltipLines(snapshot, config).join("");
  };

  render(initialMetrics);
  window.addEventListener("app-metrics", (event) => render(event.detail));
  window.addEventListener("app-config", (event) => {
    config = event.detail;
    render(currentMetrics);
  });
}

function tooltipLines(snapshot, config) {
  return [
    line("CPU", percent(snapshot.cpu_percent)),
    line("MEM", `${bytes(snapshot.memory.used_bytes)} / ${bytes(snapshot.memory.total_bytes)} (${percent(snapshot.memory.percent)})`),
    line("GPU", gpuText(snapshot, config)),
    line("NET", `↓ ${shortSpeed(snapshot.network.download_bps, "auto")}  ↑ ${shortSpeed(snapshot.network.upload_bps, "auto")}`)
  ];
}

function gpuText(snapshot, config) {
  const gpu = snapshot.gpu || {};
  const usage = gpu.usage_percent == null ? (config.show_na ? "N/A" : "") : percent(gpu.usage_percent);
  const name = shortGpuName(gpu.name);
  const vram = vramText(gpu);
  return [usage, name, vram].filter(Boolean).join(" · ");
}

function vramText(gpu) {
  if (gpu.memory_used_bytes == null || gpu.memory_total_bytes == null) return "";
  return `${bytes(gpu.memory_used_bytes).replace(" GB", "G")} / ${bytes(gpu.memory_total_bytes).replace(" GB", "G")}`;
}

function shortGpuName(name) {
  if (!name) return "";
  const cleaned = name
    .replace(/NVIDIA/gi, "")
    .replace(/GeForce/gi, "")
    .replace(/Laptop GPU/gi, "")
    .replace(/\bGPU\b/gi, "")
    .trim();
  const modelPart = cleaned.split(/\s+/).find((part) => /\d/.test(part) && part.length >= 3);
  return modelPart || cleaned;
}

function line(label, value) {
  return `<div class="tooltip-line"><span>${label}</span><strong>${escapeHtml(value)}</strong></div>`;
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
