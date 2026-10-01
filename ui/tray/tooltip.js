import {
  LEVEL_TEXT,
  OVERALL_TEXT,
  compactBytes,
  escapeHtml,
  levelOf,
  overallDetail,
  overallLevel,
  percent,
  shortGpuName,
  speed
} from "../components/format.js";
import { morph } from "../components/morph.js";
import { ring } from "../components/ring.js";

export function renderTooltip(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  document.documentElement.classList.add("transparent-window");
  root.className = "tip-shell";

  const render = (snapshot) => {
    if (snapshot) currentMetrics = snapshot;
    morph(root, tooltipMarkup(currentMetrics));
  };

  render(initialMetrics);
  window.addEventListener("app-metrics", (event) => render(event.detail));
  window.addEventListener("app-config", (event) => {
    config = event.detail;
    render();
  });
}

function tooltipMarkup(snapshot) {
  const level = overallLevel(snapshot);
  const gpu = snapshot?.gpu || {};
  const memory = snapshot?.memory || {};
  const gpuDetail = [
    shortGpuName(gpu.name),
    gpu.memory_used_bytes != null && gpu.memory_total_bytes != null
      ? `${compactBytes(gpu.memory_used_bytes)}/${compactBytes(gpu.memory_total_bytes)}`
      : "",
    gpu.temperature_celsius != null ? `${Math.round(gpu.temperature_celsius)}°C` : ""
  ].filter(Boolean).join(" · ");

  return `
    <section class="tip level-${level}">
      <header class="tip-head">
        ${ring({ percent: snapshot?.focus_percent ?? 0, level, size: 34, stroke: 4.5 })}
        <div class="tip-title">
          <strong>${OVERALL_TEXT[level]}</strong>
          <span>${escapeHtml(overallDetail(snapshot))}</span>
        </div>
      </header>
      ${row("CPU", "", snapshot?.cpu_percent, levelOf(snapshot, "cpu"))}
      ${row("内存", `${compactBytes(memory.used_bytes)} / ${compactBytes(memory.total_bytes)}`, memory.percent, levelOf(snapshot, "memory"))}
      ${row("GPU", gpuDetail, gpu.usage_percent, gpu.usage_percent == null ? "na" : levelOf(snapshot, "gpu"))}
      <div class="tip-net">
        <span class="tip-label">网络</span>
        <span class="tip-speed"><i>↓</i>${speed(snapshot?.network?.download_bps)}</span>
        <span class="tip-speed"><i>↑</i>${speed(snapshot?.network?.upload_bps)}</span>
      </div>
    </section>
  `;
}

function row(label, detail, value, level) {
  const width = Number.isFinite(value) ? Math.min(100, Math.max(0, value)) : 0;
  const levelText = level === "na" ? "" : LEVEL_TEXT[level];
  return `
    <div class="tip-row level-${level}">
      <div class="tip-line">
        <span class="tip-label">${label}</span>
        <span class="tip-detail">${escapeHtml(detail)}</span>
        <strong class="tip-value" title="${levelText}">${percent(value)}</strong>
      </div>
      <div class="meter"><span style="width:${width.toFixed(1)}%"></span></div>
    </div>
  `;
}
