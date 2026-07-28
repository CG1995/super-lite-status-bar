export async function renderSettings(root, api, config) {
  const platform = await api.getPlatform().catch(() => "windows");
  const isWindows = platform === "windows";
  const platformLabel = isWindows ? "Windows 托盘" : "macOS 菜单栏";

  root.className = "settings-shell";
  root.innerHTML = `
    <form class="settings" data-settings>
      <header class="settings-header">
        <div>
          <h1>PulseRing</h1>
          <p>${platformLabel}状态监测，保持安静、清晰和轻量。</p>
        </div>
        <span class="autosave-state" data-status>自动保存</span>
      </header>

      ${section("启动", "控制 PulseRing 是否随系统启动。", `
        ${toggle("autostart", "开机自启动", "登录后自动在后台启动，保持托盘 / 菜单栏入口可用。", config.autostart)}
      `)}

      ${section("外观", "统一设置页、状态弹窗和悬浮窗的基础视觉风格。", `
        ${select("theme", "主题", "建议跟随系统，让应用自然贴合当前桌面环境。", config.theme, [["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"]])}
      `)}

      ${section("显示规则", "状态信息使用固定的小字号、1 秒刷新、自动网络单位和 N/A 占位，避免桌面常驻信息忽大忽小。", `
        <div class="display-rules" aria-label="Display rules">
          ${rule("字号", "小号")}
          ${rule("刷新", "1 秒")}
          ${rule("网速", "自动单位")}
          ${rule("缺失数据", "N/A")}
        </div>
      `)}

      ${isWindows ? section("悬浮窗", "用于在桌面上常驻展示核心状态；高级行为集中在这里，避免悬浮窗自身变复杂。", `
        ${toggle("floating_bar.enabled", "开启悬浮窗", "显示迷你状态条，可拖动到屏幕任意位置。", config.floating_bar.enabled)}
        ${range("floating_bar.opacity", "透明度", "降低透明度可减少遮挡，但不要低到影响可读性。", config.floating_bar.opacity, 0.35, 1, 0.05)}
        ${toggle("floating_bar.always_on_top", "保持置顶", "让悬浮窗位于普通窗口上方。", config.floating_bar.always_on_top)}
        ${toggle("floating_bar.lock_position", "锁定位置", "锁定后避免误拖动，悬浮窗上方会显示固定状态。", config.floating_bar.lock_position)}
        ${toggle("floating_bar.click_through", "点击穿透", "仅在锁定位置时生效，适合放在工作区边缘。", config.floating_bar.click_through)}
        <div class="section-actions">
          <button type="button" data-reset-floating>恢复默认位置</button>
        </div>
      `) : ""}

      <footer class="settings-footer">
        <button type="button" data-reset>恢复默认设置</button>
        <button type="button" data-logs>打开日志目录</button>
        <button type="button" data-quit>退出应用</button>
      </footer>
    </form>
  `;

  const form = root.querySelector("[data-settings]");
  const status = root.querySelector("[data-status]");
  let saveTimer = null;

  api.getAutostart().then((enabled) => {
    config.autostart = enabled;
    const autostart = form.elements.namedItem("autostart");
    if (autostart) autostart.checked = enabled;
  }).catch((error) => {
    status.textContent = `读取自启动失败：${String(error)}`;
  });

  if (window.__settingsConfigListener) {
    window.removeEventListener("app-config", window.__settingsConfigListener);
  }
  window.__settingsConfigListener = (event) => {
    config = normalizeFixedConfig(event.detail);
    applyConfigToForm(form, config);
  };
  window.addEventListener("app-config", window.__settingsConfigListener);

  const scheduleSave = (delay = 120) => {
    clearTimeout(saveTimer);
    status.textContent = "保存中...";
    saveTimer = setTimeout(async () => {
      await saveCurrent();
    }, delay);
  };

  const saveCurrent = async () => {
    try {
      const next = normalizeFixedConfig(readConfig(form, config));
      if (next.autostart !== config.autostart) {
        await api.setAutostart(next.autostart);
      }
      config = await api.saveConfig(next);
      status.textContent = "已自动保存";
    } catch (error) {
      status.textContent = `自动保存失败：${String(error)}`;
    }
  };

  form.addEventListener("change", () => scheduleSave(80));
  form.addEventListener("input", (event) => {
    if (event.target?.matches?.("input[type='range']")) {
      syncRangeValue(event.target);
      scheduleSave(200);
    }
  });

  root.querySelector("[data-reset]").addEventListener("click", async () => {
    config = await api.resetConfig();
    renderSettings(root, api, config);
  });

  root.querySelector("[data-reset-floating]")?.addEventListener("click", async () => {
    config = await api.resetFloatingPosition();
    status.textContent = "悬浮窗位置已恢复";
  });

  root.querySelector("[data-logs]").addEventListener("click", async () => {
    const path = await api.showLogFolder();
    status.textContent = path ? `日志目录：${path}` : "日志目录已打开";
  });

  root.querySelector("[data-quit]").addEventListener("click", () => api.quit());
}

function section(title, description, content) {
  return `
    <section class="settings-section">
      <div class="section-heading">
        <h2>${title}</h2>
        <p>${description}</p>
      </div>
      <div class="section-body">
        ${content}
      </div>
    </section>
  `;
}

function toggle(name, label, description, checked) {
  return `
    <label class="field inline">
      <span class="field-copy">
        <span class="field-label">${label}</span>
        <span class="field-description">${description}</span>
      </span>
      <input class="toggle-control" type="checkbox" name="${name}" ${checked ? "checked" : ""} />
    </label>
  `;
}

function select(name, label, description, value, options) {
  return `
    <label class="field">
      <span class="field-copy">
        <span class="field-label">${label}</span>
        <span class="field-description">${description}</span>
      </span>
      <select name="${name}">
        ${options.map(([optionValue, text]) => `<option value="${optionValue}" ${String(value) === optionValue ? "selected" : ""}>${text}</option>`).join("")}
      </select>
    </label>
  `;
}

function range(name, label, description, value, min, max, step) {
  return `
    <label class="field">
      <span class="field-copy">
        <span class="field-label">${label}</span>
        <span class="field-description">${description}</span>
      </span>
      <span class="range-control">
        <input type="range" name="${name}" value="${value}" min="${min}" max="${max}" step="${step}" />
        <output data-range-value="${name}">${Math.round(Number(value) * 100)}%</output>
      </span>
    </label>
  `;
}

function rule(label, value) {
  return `
    <span class="display-rule">
      <span>${label}</span>
      <strong>${value}</strong>
    </span>
  `;
}

function readConfig(form, config) {
  const next = JSON.parse(JSON.stringify(config));
  for (const element of form.elements) {
    if (!element.name) continue;
    const value = element.type === "checkbox"
      ? element.checked
      : element.type === "range"
        ? Number(element.value)
        : element.value;
    setByPath(next, element.name, value);
  }
  return next;
}

function applyConfigToForm(form, config) {
  for (const element of form.elements) {
    if (!element.name) continue;
    const value = getByPath(config, element.name);
    if (value === undefined) continue;
    if (element.type === "checkbox") {
      element.checked = Boolean(value);
    } else {
      element.value = String(value);
    }
    if (element.type === "range") syncRangeValue(element);
  }
}

function syncRangeValue(input) {
  const output = input.form?.querySelector(`[data-range-value="${input.name}"]`);
  if (output) output.textContent = `${Math.round(Number(input.value) * 100)}%`;
}

function normalizeFixedConfig(config) {
  config.refresh_interval_ms = 1000;
  config.font = { preset: "small", custom_px: 12 };
  config.speed_unit = "auto";
  config.show_na = true;
  return config;
}

function setByPath(target, path, value) {
  const parts = path.split(".");
  let cursor = target;
  while (parts.length > 1) {
    const part = parts.shift();
    cursor = cursor[part];
  }
  cursor[parts[0]] = value;
}

function getByPath(target, path) {
  return path.split(".").reduce((cursor, part) => cursor?.[part], target);
}
