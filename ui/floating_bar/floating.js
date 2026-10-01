import { levelOf, overallLevel, percent, shortSpeed } from "../components/format.js";
import { morph } from "../components/morph.js";
import { ring } from "../components/ring.js";

const SHELL_PADDING = 12;

const LOCK_ICON = `
  <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="3.5" y="7" width="9" height="6.5" rx="1.6"/><path d="M5.5 7V5.2a2.5 2.5 0 0 1 5 0V7"/></svg>`;
const GRIP_ICON = `
  <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="6" cy="4" r="1.1"/><circle cx="10" cy="4" r="1.1"/><circle cx="6" cy="8" r="1.1"/><circle cx="10" cy="8" r="1.1"/><circle cx="6" cy="12" r="1.1"/><circle cx="10" cy="12" r="1.1"/></svg>`;

/** Markup for the bar itself; also used by the live preview in the control center. */
export function floatingMarkup(snapshot, config, { interactive = true } = {}) {
  const floating = config.floating_bar || {};
  const locked = Boolean(floating.lock_position);
  const level = overallLevel(snapshot);
  const metrics = [
    metric("CPU", percent(snapshot?.cpu_percent), levelOf(snapshot, "cpu")),
    floating.show_memory !== false
      ? metric("内存", percent(snapshot?.memory?.percent), levelOf(snapshot, "memory"))
      : "",
    floating.show_gpu !== false
      ? metric("GPU", gpuValue(snapshot), snapshot?.gpu?.usage_percent == null ? "na" : levelOf(snapshot, "gpu"))
      : "",
    floating.show_network !== false
      ? `<span class="fb-metric fb-net"><i>↓</i><b>${shortSpeed(snapshot?.network?.download_bps)}</b><i>↑</i><b>${shortSpeed(snapshot?.network?.upload_bps)}</b></span>`
      : ""
  ].filter(Boolean);

  return `
    <div class="fb level-${level} ${locked ? "is-locked" : ""}" style="--alpha:${Number(floating.opacity ?? 0.85)}">
      <span class="fb-pulse" title="整体状态">${ring({ percent: snapshot?.focus_percent ?? 0, level, size: 16, stroke: 3 })}</span>
      ${metrics.join('<span class="fb-sep"></span>')}
      ${interactive ? `
        <button class="fb-control" type="button" data-fb-control
          title="${locked ? "已锁定，点击解锁" : "拖动以移动，点击锁定位置"}"
          aria-label="${locked ? "解锁位置" : "锁定位置"}" aria-pressed="${locked}">
          ${locked ? LOCK_ICON : GRIP_ICON}
        </button>` : ""}
    </div>
  `;
}

function metric(label, value, level) {
  return `<span class="fb-metric level-${level}"><i>${label}</i><b>${value}</b></span>`;
}

function gpuValue(snapshot) {
  const usage = snapshot?.gpu?.usage_percent;
  return usage == null ? "N/A" : percent(usage);
}

export function renderFloatingBar(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  let userDragging = false;
  let persistTimer = null;
  let fitFrame = 0;
  let lastFit = "";

  document.documentElement.classList.add("transparent-window");
  root.className = "fb-shell";
  root.style.padding = `${SHELL_PADDING}px`;

  const fitWindow = () => {
    cancelAnimationFrame(fitFrame);
    fitFrame = requestAnimationFrame(() => {
      const bar = root.querySelector(".fb");
      if (!bar) return;
      const rect = bar.getBoundingClientRect();
      const control = root.querySelector("[data-fb-control]")?.getBoundingClientRect();
      const hotZone = control
        ? { x: control.left, y: control.top, width: control.width, height: control.height }
        : null;
      const width = Math.ceil(rect.width + SHELL_PADDING * 2);
      const height = Math.ceil(rect.height + SHELL_PADDING * 2);
      const key = JSON.stringify([width, height, hotZone]);
      if (key === lastFit) return;
      lastFit = key;
      api.fitFloating(width, height, hotZone).catch(() => {
        lastFit = "";
      });
    });
  };

  const render = (snapshot) => {
    if (snapshot) currentMetrics = snapshot;
    morph(root, floatingMarkup(currentMetrics, config));
    root.classList.toggle("is-click-through", Boolean(config.floating_bar.lock_position && config.floating_bar.click_through));
    fitWindow();
  };

  render(initialMetrics);
  window.addEventListener("app-metrics", (event) => render(event.detail));
  window.addEventListener("app-config", (event) => {
    config = event.detail;
    render();
  });
  // Font loading can change the measured width after first paint.
  document.fonts?.ready?.then(fitWindow);

  api.onMoved(() => {
    if (!userDragging) return;
    clearTimeout(persistTimer);
    persistTimer = setTimeout(async () => {
      userDragging = false;
      try {
        config = await api.persistFloatingPosition();
      } catch {
        // Position persistence is best effort.
      }
    }, 450);
  });

  // Start the native drag only once the pointer actually moves: the OS move loop
  // swallows mouseup, which would otherwise make double-click impossible.
  let pressPoint = null;
  root.addEventListener("mousedown", (event) => {
    if (event.button !== 0 || event.target.closest("[data-fb-control]")) return;
    pressPoint = config.floating_bar.lock_position ? null : [event.screenX, event.screenY];
  });
  root.addEventListener("mousemove", async (event) => {
    if (!pressPoint || (event.buttons & 1) === 0) {
      pressPoint = null;
      return;
    }
    const distance = Math.hypot(event.screenX - pressPoint[0], event.screenY - pressPoint[1]);
    if (distance < 3) return;
    pressPoint = null;
    userDragging = true;
    await api.startDragging();
  });
  root.addEventListener("mouseup", () => {
    pressPoint = null;
  });

  root.addEventListener("dblclick", (event) => {
    if (event.target.closest("[data-fb-control]")) return;
    api.showSettings();
  });

  root.addEventListener("click", async (event) => {
    if (!event.target.closest("[data-fb-control]")) return;
    event.stopPropagation();
    const next = JSON.parse(JSON.stringify(await api.persistFloatingPosition()));
    next.floating_bar.lock_position = !next.floating_bar.lock_position;
    config = await api.saveConfig(next);
    render();
  });

  root.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    api.showFloatingMenu().catch(() => {});
  });
}
