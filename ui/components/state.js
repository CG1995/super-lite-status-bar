const fallbackConfig = {
  autostart: false,
  launch_hidden: true,
  display_mode: "compact",
  refresh_interval_ms: 1000,
  font: { preset: "small", custom_px: 12 },
  speed_unit: "auto",
  temperature_unit: "celsius",
  indicators: {
    cpu: true,
    memory: true,
    gpu: true,
    network_upload: true,
    network_download: true
  },
  floating_bar: {
    enabled: false,
    opacity: 0.92,
    always_on_top: true,
    lock_position: false,
    click_through: false,
    layout: "horizontal"
  },
  theme: "system",
  show_na: true,
  macos_text_enabled: true,
  macos_max_text_chars: 34
};

export function createApi() {
  const tauri = window.__TAURI__;
  const invoke = tauri?.core?.invoke;
  const listen = tauri?.event?.listen;

  if (!invoke) {
    return createMockApi();
  }

  return {
    isTauri: true,
    invoke,
    getConfig: () => invoke("get_config"),
    saveConfig: (config) => invoke("save_config", { config }),
    resetConfig: () => invoke("reset_config"),
    getMetrics: () => invoke("get_latest_metrics"),
    getAutostart: () => invoke("get_autostart"),
    getPlatform: () => invoke("get_platform"),
    setAutostart: (enabled) => invoke("set_autostart", { enabled }),
    showSettings: () => invoke("show_settings"),
    resetFloatingPosition: () => invoke("reset_floating_position"),
    persistFloatingPosition: () => invoke("persist_floating_position"),
    showLogFolder: () => invoke("show_log_folder"),
    quit: () => invoke("quit_app"),
    listen: async (event, handler) => {
      if (!listen) return () => {};
      return listen(event, (payload) => handler(payload.payload));
    },
    hideCurrentWindow: async () => {
      await invoke("hide_current_window");
    },
    startDragging: async () => {
      const win = tauri?.window?.getCurrentWindow?.();
      if (win?.startDragging) await win.startDragging();
    }
  };
}

function createMockApi() {
  let config = clone(fallbackConfig);
  let metrics = mockMetrics();
  setInterval(() => {
    metrics = mockMetrics();
    window.dispatchEvent(new CustomEvent("app-metrics", { detail: metrics }));
  }, 1000);

  return {
    isTauri: false,
    getConfig: async () => config,
    saveConfig: async (next) => {
      config = clone(next);
      window.dispatchEvent(new CustomEvent("app-config", { detail: config }));
      return config;
    },
    resetConfig: async () => {
      config = clone(fallbackConfig);
      return config;
    },
    getMetrics: async () => metrics,
    getAutostart: async () => config.autostart,
    getPlatform: async () => "windows",
    setAutostart: async (enabled) => {
      config.autostart = enabled;
      return enabled;
    },
    showSettings: async () => {},
    resetFloatingPosition: async () => config,
    persistFloatingPosition: async () => config,
    showLogFolder: async () => "",
    quit: async () => {},
    listen: async () => () => {},
    hideCurrentWindow: async () => {},
    startDragging: async () => {}
  };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function mockMetrics() {
  const cpu = 8 + Math.random() * 52;
  const mem = 42 + Math.random() * 20;
  const down = Math.random() * 4 * 1024 * 1024;
  const up = Math.random() * 700 * 1024;
  const pressure = Math.max(cpu, mem) > 65 ? "medium" : "normal";
  return {
    cpu_percent: cpu,
    memory: {
      used_bytes: 8.1 * 1024 * 1024 * 1024,
      total_bytes: 16 * 1024 * 1024 * 1024,
      percent: mem
    },
    network: {
      download_bps: down,
      upload_bps: up
    },
    gpu: {
      name: "NVIDIA GeForce RTX 3070 Laptop GPU",
      usage_percent: 10 + Math.random() * 30,
      memory_used_bytes: 2.8 * 1024 * 1024 * 1024,
      memory_total_bytes: 8 * 1024 * 1024 * 1024,
      temperature_celsius: null,
      available: true
    },
    pressure,
    compact_text: `CPU ${Math.round(cpu)}% | MEM ${Math.round(mem)}% | ↓ ${(down / 1024 / 1024).toFixed(1)}M | ↑ ${Math.round(up / 1024)}K`,
    full_text: "",
    tooltip: ""
  };
}
