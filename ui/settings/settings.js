export async function renderSettings(root, api, config) {
  const platform = await api.getPlatform().catch(() => "windows");
  const isWindows = platform === "windows";

  root.className = "settings-shell";
  root.innerHTML = `
    <form class="settings" data-settings>
      <header class="settings-header">
        <div>
          <h1>${isWindows ? "脉环" : "PulseRing"}</h1>
          <p>${isWindows ? "Windows 托盘状态监测" : "macOS 菜单栏状态监测"}</p>
        </div>
        <span class="autosave-state" data-status>自动保存</span>
      </header>

      <section class="settings-section">
        <h2>启动</h2>
        ${toggle("autostart", "开机自启动", config.autostart)}
      </section>

      <section class="settings-section">
        <h2>外观</h2>
        ${select("theme", "主题", config.theme, [["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"]])}
      </section>

      ${isWindows ? `<section class="settings-section">
        <h2>悬浮窗</h2>
        ${toggle("floating_bar.enabled", "开启", config.floating_bar.enabled)}
        ${range("floating_bar.opacity", "透明度", config.floating_bar.opacity, 0.35, 1, 0.05)}
        ${toggle("floating_bar.always_on_top", "置顶", config.floating_bar.always_on_top)}
        ${toggle("floating_bar.lock_position", "锁定位置", config.floating_bar.lock_position)}
        ${toggle("floating_bar.click_through", "点击穿透", config.floating_bar.click_through)}
        <button type="button" data-reset-floating>恢复默认位置</button>
      </section>` : ""}

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
      scheduleSave(200);
    }
  });

  root.querySelector("[data-reset]").addEventListener("click", async () => {
    config = await api.resetConfig();
    renderSettings(root, api, config);
  });

  root.querySelector("[data-reset-floating]")?.addEventListener("click", async () => {
    config = await api.resetFloatingPosition();
    status.textContent = "悬浮条位置已恢复";
  });

  root.querySelector("[data-logs]").addEventListener("click", async () => {
    const path = await api.showLogFolder();
    status.textContent = path ? `日志目录：${path}` : "日志目录已打开";
  });

  root.querySelector("[data-quit]").addEventListener("click", () => api.quit());
}

function toggle(name, label, checked) {
  return `
    <label class="field inline">
      <span>${label}</span>
      <input type="checkbox" name="${name}" ${checked ? "checked" : ""} />
    </label>
  `;
}

function select(name, label, value, options) {
  return `
    <label class="field">
      <span>${label}</span>
      <select name="${name}">
        ${options.map(([optionValue, text]) => `<option value="${optionValue}" ${String(value) === optionValue ? "selected" : ""}>${text}</option>`).join("")}
      </select>
    </label>
  `;
}

function range(name, label, value, min, max, step) {
  return `
    <label class="field">
      <span>${label}</span>
      <input type="range" name="${name}" value="${value}" min="${min}" max="${max}" step="${step}" />
    </label>
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
  }
}

function normalizeFixedConfig(config) {
  config.launch_hidden = true;
  config.display_mode = "compact";
  config.refresh_interval_ms = 1000;
  config.font = { preset: "small", custom_px: 12 };
  config.speed_unit = "auto";
  config.temperature_unit = "celsius";
  config.indicators = {
    cpu: true,
    memory: true,
    gpu: true,
    network_upload: true,
    network_download: true
  };
  config.show_na = true;
  config.macos_text_enabled = true;
  config.macos_max_text_chars = 34;
  config.floating_bar.layout = "horizontal";
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
