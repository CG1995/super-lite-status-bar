import {
  LEVEL_TEXT,
  OVERALL_TEXT,
  THRESHOLDS,
  bytes,
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
import { ring, sparkline } from "../components/ring.js";
import { floatingMarkup } from "../floating_bar/floating.js";

const HISTORY = 60;
const PAGE_KEY = "pulsering.page";

const PAGES = [
  ["overview", "概览", `<path d="M3 13a7 7 0 1 1 14 0"/><path d="M10 13l3.5-4"/>`],
  ["floating", "悬浮条", `<rect x="2.5" y="7" width="15" height="6" rx="3"/>`],
  ["alerts", "状态提醒", `<path d="M10 3a5 5 0 0 0-5 5v3l-1.5 2.5h13L15 11V8a5 5 0 0 0-5-5z"/><path d="M8 16a2 2 0 0 0 4 0"/>`],
  ["general", "通用", `<circle cx="10" cy="10" r="2.5"/><path d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M4.7 15.3l1.4-1.4M13.9 6.1l1.4-1.4"/>`]
];

export async function renderSettings(root, api, config, initialMetrics) {
  const [platform, version] = await Promise.all([
    api.getPlatform().catch(() => "windows"),
    api.getVersion().catch(() => "")
  ]);
  const isWindows = platform === "windows";
  const pages = PAGES.filter(([id]) => isWindows || id !== "floating");
  const history = { cpu: [], memory: [], gpu: [], down: [] };
  let metrics = initialMetrics;
  let page = readPage(pages);

  root.className = "cc-shell";
  root.innerHTML = `
    <div class="cc">
      <aside class="cc-side">
        <div class="brand">
          <span class="brand-mark" data-brand-ring></span>
          <div><strong>PulseRing</strong><span>${version ? `v${escapeHtml(version)}` : "脉环"}</span></div>
        </div>
        <nav class="cc-nav" aria-label="页面">
          ${pages.map(([id, label, icon]) => `
            <button type="button" class="nav-item" data-page="${id}" aria-current="${id === page}">
              <svg viewBox="0 0 20 20" aria-hidden="true">${icon}</svg><span>${label}</span>
            </button>`).join("")}
        </nav>
        <div class="side-foot">
          <div class="side-status" data-side-status></div>
        </div>
      </aside>

      <form class="cc-main" data-settings autocomplete="off">
        <header class="page-head">
          <h1 data-page-title></h1>
          <span class="save-state" data-status role="status" aria-live="polite"></span>
        </header>

        <section class="page" data-page-panel="overview">
          <div data-overview></div>
        </section>

        ${isWindows ? floatingPage(config) : ""}
        ${alertsPage(config)}
        ${generalPage(config)}
      </form>
    </div>
  `;

  const form = root.querySelector("[data-settings]");
  const status = root.querySelector("[data-status]");
  let statusTimer = null;

  const setStatus = (text, tone = "ok") => {
    status.textContent = text;
    status.dataset.tone = tone;
    status.classList.add("is-visible");
    clearTimeout(statusTimer);
    if (tone === "ok") {
      statusTimer = setTimeout(() => status.classList.remove("is-visible"), 1800);
    }
  };

  const showPage = (id) => {
    page = id;
    try {
      localStorage.setItem(PAGE_KEY, id);
    } catch {
      // Remembering the page is a convenience only.
    }
    root.querySelectorAll("[data-page]").forEach((button) => {
      button.setAttribute("aria-current", String(button.dataset.page === id));
    });
    root.querySelectorAll("[data-page-panel]").forEach((panel) => {
      panel.hidden = panel.dataset.pagePanel !== id;
    });
    root.querySelector("[data-page-title]").textContent = pages.find(([key]) => key === id)?.[1] || "";
    form.scrollTop = 0;
  };

  const renderLive = () => {
    const level = overallLevel(metrics);
    morph(root.querySelector("[data-brand-ring]"), ring({ percent: metrics?.focus_percent ?? 0, level, size: 30, stroke: 4 }));
    morph(root.querySelector("[data-side-status]"), `
      <span class="dot level-${level}"></span>
      <span><strong>${OVERALL_TEXT[level]}</strong><small>${escapeHtml(overallDetail(metrics))}</small></span>
    `);
    morph(root.querySelector("[data-overview]"), overviewMarkup(metrics, history, isWindows, config));
    const preview = root.querySelector("[data-preview]");
    if (preview) morph(preview, floatingMarkup(metrics, config, { interactive: false }));
  };

  const pushHistory = (snapshot) => {
    if (!snapshot) return;
    const push = (key, value) => {
      history[key].push(Number.isFinite(value) ? value : 0);
      if (history[key].length > HISTORY) history[key].shift();
    };
    push("cpu", snapshot.cpu_percent);
    push("memory", snapshot.memory?.percent);
    push("gpu", snapshot.gpu?.usage_percent);
    push("down", snapshot.network?.download_bps);
  };

  const syncDependentControls = () => {
    const lock = form.elements.namedItem("floating_bar.lock_position");
    const clickThrough = form.elements.namedItem("floating_bar.click_through");
    if (lock && clickThrough) {
      clickThrough.disabled = !lock.checked;
      clickThrough.closest(".field")?.classList.toggle("is-disabled", !lock.checked);
    }
    const alert = form.querySelector("input[name='alert']:checked")?.value || config.alert;
    const table = root.querySelector("[data-thresholds]");
    if (table) table.innerHTML = thresholdRows(alert);
    root.querySelectorAll("input[type='range']").forEach(syncRangeValue);
  };

  pushHistory(metrics);
  showPage(page);
  renderLive();
  syncDependentControls();

  api.getAutostart().then((enabled) => {
    config.autostart = enabled;
    const autostart = form.elements.namedItem("autostart");
    if (autostart) autostart.checked = enabled;
  }).catch((error) => setStatus(`读取自启动状态失败：${String(error)}`, "error"));

  window.addEventListener("app-metrics", (event) => {
    metrics = event.detail;
    pushHistory(metrics);
    renderLive();
  });

  window.addEventListener("app-config", (event) => {
    config = event.detail;
    applyConfigToForm(form, config);
    syncDependentControls();
    renderLive();
  });

  root.querySelectorAll("[data-page]").forEach((button) => {
    button.addEventListener("click", () => showPage(button.dataset.page));
  });

  let saveTimer = null;
  const save = async () => {
    try {
      const next = readConfig(form, config);
      if (next.autostart !== config.autostart) {
        await api.setAutostart(next.autostart);
      }
      config = await api.saveConfig(next);
      setStatus("已保存");
    } catch (error) {
      setStatus(`保存失败：${String(error)}`, "error");
      applyConfigToForm(form, config);
    }
    syncDependentControls();
    renderLive();
  };

  form.addEventListener("change", () => {
    syncDependentControls();
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 60);
  });
  form.addEventListener("input", (event) => {
    if (!event.target?.matches?.("input[type='range']")) return;
    syncRangeValue(event.target);
    // Live preview while dragging; persist shortly after the thumb settles.
    config = readConfig(form, config);
    renderLive();
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 250);
  });
  form.addEventListener("submit", (event) => event.preventDefault());

  root.querySelector("[data-reset-floating]")?.addEventListener("click", async () => {
    config = await api.resetFloatingPosition();
    setStatus("悬浮条已移回右下角");
  });
  const resetButton = root.querySelector("[data-reset]");
  let resetArmed = null;
  resetButton.addEventListener("click", async () => {
    if (!resetArmed) {
      resetButton.textContent = "再点一次确认";
      resetButton.classList.add("btn-danger");
      resetArmed = setTimeout(() => {
        resetArmed = null;
        resetButton.textContent = "恢复默认设置";
        resetButton.classList.remove("btn-danger");
      }, 3000);
      return;
    }
    clearTimeout(resetArmed);
    resetArmed = null;
    resetButton.textContent = "恢复默认设置";
    resetButton.classList.remove("btn-danger");
    config = await api.resetConfig();
    if (config.autostart !== (await api.getAutostart().catch(() => config.autostart))) {
      await api.setAutostart(config.autostart).catch(() => {});
    }
    applyConfigToForm(form, config);
    syncDependentControls();
    renderLive();
    setStatus("已恢复默认设置");
  });
  root.querySelector("[data-logs]").addEventListener("click", async () => {
    const path = await api.showLogFolder();
    setStatus(path ? `已打开：${path}` : "已打开日志目录");
  });
  root.querySelector("[data-quit]").addEventListener("click", () => api.quit());
}

function readPage(pages) {
  try {
    const saved = localStorage.getItem(PAGE_KEY);
    if (pages.some(([id]) => id === saved)) return saved;
  } catch {
    // Storage can be unavailable; fall back to the overview.
  }
  return "overview";
}

/* ---------- Overview ---------- */

function overviewMarkup(snapshot, history, isWindows, config) {
  const level = overallLevel(snapshot);
  const gpu = snapshot?.gpu || {};
  const memory = snapshot?.memory || {};
  const gpuAvailable = gpu.usage_percent != null;
  const floatingOn = config.floating_bar?.enabled;

  return `
    <div class="hero level-${level}">
      ${ring({ percent: snapshot?.focus_percent ?? 0, level, size: 76, stroke: 8, className: "hero-ring" })}
      <div class="hero-copy">
        <span class="chip level-${level}">${LEVEL_TEXT[level]}</span>
        <h2>${OVERALL_TEXT[level]}</h2>
        <p>${escapeHtml(overallDetail(snapshot))}</p>
      </div>
      ${isWindows ? `<span class="hero-hint">${floatingOn ? "悬浮条已开启" : "悬浮条已关闭"} · 托盘图标颜色同步状态</span>` : ""}
    </div>
    <div class="cards">
      ${card({
        title: "CPU",
        value: percent(snapshot?.cpu_percent),
        percentValue: snapshot?.cpu_percent,
        level: levelOf(snapshot, "cpu"),
        detail: "处理器总占用",
        spark: sparkline(history.cpu)
      })}
      ${card({
        title: "内存",
        value: percent(memory.percent),
        percentValue: memory.percent,
        level: levelOf(snapshot, "memory"),
        detail: `${bytes(memory.used_bytes)} / ${bytes(memory.total_bytes)}`,
        spark: sparkline(history.memory)
      })}
      ${card({
        title: "GPU",
        value: gpuAvailable ? percent(gpu.usage_percent) : "N/A",
        percentValue: gpu.usage_percent,
        level: gpuAvailable ? levelOf(snapshot, "gpu") : "na",
        detail: gpuAvailable
          ? [shortGpuName(gpu.name), gpu.memory_total_bytes != null ? `显存 ${compactBytes(gpu.memory_used_bytes)}/${compactBytes(gpu.memory_total_bytes)}` : "", gpu.temperature_celsius != null ? `${Math.round(gpu.temperature_celsius)}°C` : ""].filter(Boolean).join(" · ")
          : "未检测到可读取的显卡数据",
        spark: sparkline(history.gpu)
      })}
      <article class="card card-net">
        <header><span class="card-title">网络</span></header>
        <div class="net-values">
          <div><i>↓ 下载</i><strong>${speed(snapshot?.network?.download_bps)}</strong></div>
          <div><i>↑ 上传</i><strong>${speed(snapshot?.network?.upload_bps)}</strong></div>
        </div>
        ${sparkline(history.down, { max: 1024 * 1024, className: "spark-net" })}
      </article>
    </div>
  `;
}

function card({ title, value, percentValue, level, detail, spark }) {
  return `
    <article class="card level-${level}">
      <header>
        <span class="card-title">${title}</span>
        ${level === "na" ? "" : `<span class="chip level-${level}">${LEVEL_TEXT[level]}</span>`}
      </header>
      <div class="card-body">
        ${ring({ percent: percentValue ?? 0, level: level === "na" ? "na" : level, size: 54, stroke: 6 })}
        <div class="card-copy">
          <strong>${value}</strong>
          <span>${escapeHtml(detail)}</span>
        </div>
      </div>
      ${spark}
    </article>
  `;
}

/* ---------- Pages ---------- */

function floatingPage(config) {
  const floating = config.floating_bar;
  return `
    <section class="page" data-page-panel="floating" hidden>
      <div class="preview-stage" aria-label="悬浮条预览">
        <div class="preview-bar" data-preview></div>
        <span class="preview-caption">实时预览 · 透明度只作用于背景，文字始终清晰</span>
      </div>

      ${group("显示", `
        ${toggle("floating_bar.enabled", "显示悬浮条", "在桌面上常驻显示核心状态。也可以在托盘菜单中快速开关。", floating.enabled)}
        ${range("floating_bar.opacity", "背景不透明度", "0% 为完全透明，只保留文字与状态色。", floating.opacity)}
        ${segmented("font.preset", "字号", "", config.font?.preset || "small", [["small", "小"], ["medium", "中"], ["large", "大"]])}
        <div class="field">
          <span class="field-copy"><span class="field-label">显示项目</span><span class="field-description">CPU 始终显示。</span></span>
          <span class="chips">
            ${chip("floating_bar.show_memory", "内存", floating.show_memory)}
            ${chip("floating_bar.show_gpu", "GPU", floating.show_gpu)}
            ${chip("floating_bar.show_network", "网络", floating.show_network)}
          </span>
        </div>
      `)}

      ${group("行为", `
        ${toggle("floating_bar.always_on_top", "保持置顶", "始终显示在其他窗口上方。", floating.always_on_top)}
        ${toggle("floating_bar.lock_position", "锁定位置", "防止误拖动。也可以点击悬浮条右侧的锁形按钮切换。", floating.lock_position)}
        ${toggle("floating_bar.click_through", "鼠标穿透", "锁定后生效：点击会穿过悬浮条落到下面的窗口，只有右侧锁形按钮保持可点。", floating.click_through)}
        <div class="field">
          <span class="field-copy"><span class="field-label">操作提示</span><span class="field-description">拖动移动 · 双击打开控制中心 · 右键打开菜单</span></span>
          <button type="button" class="btn" data-reset-floating>移回右下角</button>
        </div>
      `)}
    </section>
  `;
}

function alertsPage(config) {
  return `
    <section class="page" data-page-panel="alerts" hidden>
      ${group("提醒灵敏度", `
        ${segmented("alert", "颜色变化的触发点", "托盘图标、悬浮条、提示卡片会按下表变色。", config.alert || "standard", [["relaxed", "宽松"], ["standard", "标准"], ["sensitive", "敏感"]])}
        <table class="thresholds">
          <thead><tr><th>指标</th><th><span class="dot level-medium"></span>偏高</th><th><span class="dot level-high"></span>告急</th></tr></thead>
          <tbody data-thresholds></tbody>
        </table>
        <p class="note"><span class="dot level-normal"></span>低于“偏高”阈值时显示为正常（绿色）。数值回落约 4% 后颜色才会恢复，避免在阈值附近来回闪烁。</p>
      `)}
      ${group("采样", `
        ${segmented("refresh_interval_ms", "刷新频率", "更快的刷新更跟手，更慢的刷新更省电。", String(config.refresh_interval_ms), [["500", "0.5 秒"], ["1000", "1 秒"], ["2000", "2 秒"], ["3000", "3 秒"]])}
      `)}
    </section>
  `;
}

function generalPage(config) {
  return `
    <section class="page" data-page-panel="general" hidden>
      ${group("启动与外观", `
        ${toggle("autostart", "开机自启动", "登录后在后台静默启动，只显示托盘图标。", config.autostart)}
        ${segmented("theme", "主题", "", config.theme || "system", [["system", "跟随系统"], ["light", "浅色"], ["dark", "深色"]])}
      `)}
      ${group("维护", `
        <div class="field">
          <span class="field-copy"><span class="field-label">日志</span><span class="field-description">排查问题时可以把日志发给开发者。</span></span>
          <button type="button" class="btn" data-logs>打开日志目录</button>
        </div>
        <div class="field">
          <span class="field-copy"><span class="field-label">恢复默认</span><span class="field-description">重置所有设置和悬浮条位置。</span></span>
          <button type="button" class="btn" data-reset>恢复默认设置</button>
        </div>
        <div class="field">
          <span class="field-copy"><span class="field-label">退出</span><span class="field-description">关闭窗口只会隐藏到托盘；要完全退出请点这里。</span></span>
          <button type="button" class="btn btn-danger" data-quit>退出 PulseRing</button>
        </div>
      `)}
    </section>
  `;
}

function thresholdRows(alert) {
  const table = THRESHOLDS[alert] || THRESHOLDS.standard;
  return [["CPU", table.cpu], ["内存", table.memory], ["GPU", table.gpu]]
    .map(([label, [warn, critical]]) => `<tr><td>${label}</td><td>≥ ${warn}%</td><td>≥ ${critical}%</td></tr>`)
    .join("");
}

/* ---------- Controls ---------- */

function group(title, content) {
  return `<div class="group"><h2>${title}</h2><div class="group-body">${content}</div></div>`;
}

function fieldCopy(label, description) {
  return `
    <span class="field-copy">
      <span class="field-label">${label}</span>
      ${description ? `<span class="field-description">${description}</span>` : ""}
    </span>`;
}

function toggle(name, label, description, checked) {
  return `
    <label class="field">
      ${fieldCopy(label, description)}
      <input class="switch" type="checkbox" role="switch" name="${name}" ${checked ? "checked" : ""} />
    </label>
  `;
}

function chip(name, label, checked) {
  return `
    <label class="chip-toggle">
      <input type="checkbox" name="${name}" ${checked !== false ? "checked" : ""} />
      <span>${label}</span>
    </label>
  `;
}

function segmented(name, label, description, value, options) {
  return `
    <div class="field">
      ${fieldCopy(label, description)}
      <span class="segmented" role="radiogroup" aria-label="${label}">
        ${options.map(([optionValue, text]) => `
          <label>
            <input type="radio" name="${name}" value="${optionValue}" ${String(value) === optionValue ? "checked" : ""} />
            <span>${text}</span>
          </label>`).join("")}
      </span>
    </div>
  `;
}

function range(name, label, description, value) {
  return `
    <label class="field">
      ${fieldCopy(label, description)}
      <span class="range-control">
        <input type="range" name="${name}" value="${value}" min="0" max="1" step="0.05" />
        <output data-range-value="${name}">${Math.round(Number(value) * 100)}%</output>
      </span>
    </label>
  `;
}

function syncRangeValue(input) {
  const output = input.form?.querySelector(`[data-range-value="${input.name}"]`);
  if (output) output.textContent = `${Math.round(Number(input.value) * 100)}%`;
  const fill = ((Number(input.value) - Number(input.min)) / (Number(input.max) - Number(input.min))) * 100;
  input.style.setProperty("--fill", `${fill}%`);
}

/* ---------- Form <-> config ---------- */

const NUMERIC_FIELDS = new Set(["refresh_interval_ms", "floating_bar.opacity"]);

function readConfig(form, config) {
  const next = JSON.parse(JSON.stringify(config));
  for (const element of form.elements) {
    if (!element.name) continue;
    if (element.type === "radio" && !element.checked) continue;
    let value = element.type === "checkbox" ? element.checked : element.value;
    if (NUMERIC_FIELDS.has(element.name)) value = Number(value);
    setByPath(next, element.name, value);
  }
  if (next.font) next.font.custom_px = next.font.custom_px || 12;
  return next;
}

function applyConfigToForm(form, config) {
  for (const element of form.elements) {
    if (!element.name) continue;
    const value = getByPath(config, element.name);
    if (value === undefined) continue;
    if (element.type === "checkbox") {
      element.checked = Boolean(value);
    } else if (element.type === "radio") {
      element.checked = String(value) === element.value;
    } else {
      element.value = String(value);
    }
  }
}

function setByPath(target, path, value) {
  const parts = path.split(".");
  let cursor = target;
  while (parts.length > 1) {
    const part = parts.shift();
    cursor[part] = cursor[part] ?? {};
    cursor = cursor[part];
  }
  cursor[parts[0]] = value;
}

function getByPath(target, path) {
  return path.split(".").reduce((cursor, part) => cursor?.[part], target);
}
