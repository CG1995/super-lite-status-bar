import { THRESHOLDS } from "./format.js";

const fallbackConfig = {
  autostart: false,
  refresh_interval_ms: 1000,
  font: { preset: "small", custom_px: 12 },
  speed_unit: "auto",
  floating_bar: {
    enabled: true,
    opacity: 0.85,
    always_on_top: true,
    lock_position: false,
    click_through: false,
    show_memory: true,
    show_gpu: true,
    show_network: true,
    x: null,
    y: null
  },
  theme: "system",
  alert: "standard",
  show_na: true
};

export function createApi() {
  const tauri = window.__TAURI__;
  const invoke = tauri?.core?.invoke;
  const listen = tauri?.event?.listen;

  if (!invoke) {
    return createMockApi();
  }

  const currentWindow = () => tauri?.window?.getCurrentWindow?.();

  return {
    isTauri: true,
    getConfig: () => invoke("get_config"),
    saveConfig: (config) => invoke("save_config", { config }),
    resetConfig: () => invoke("reset_config"),
    getMetrics: () => invoke("get_latest_metrics"),
    getAutostart: () => invoke("get_autostart"),
    getPlatform: () => invoke("get_platform"),
    getVersion: () => invoke("get_app_version"),
    setAutostart: (enabled) => invoke("set_autostart", { enabled }),
    showSettings: () => invoke("show_settings"),
    resetFloatingPosition: () => invoke("reset_floating_position"),
    persistFloatingPosition: () => invoke("persist_floating_position"),
    fitFloating: (width, height, hotZone) =>
      invoke("fit_floating_window", { width, height, hotZone }),
    showFloatingMenu: () => invoke("show_floating_menu"),
    showLogFolder: () => invoke("show_log_folder"),
    quit: () => invoke("quit_app"),
    listen: async (event, handler) => {
      if (!listen) return () => {};
      return listen(event, (payload) => handler(payload.payload));
    },
    onMoved: async (handler) => {
      const win = currentWindow();
      return win?.onMoved ? win.onMoved(handler) : () => {};
    },
    startDragging: async () => {
      const win = currentWindow();
      if (win?.startDragging) await win.startDragging();
    }
  };
}

/** Browser preview mode: lets the UI be designed and checked without the Tauri shell. */
function createMockApi() {
  let config = clone(fallbackConfig);
  const params = new URLSearchParams(window.location.search);
  const forced = params.get("level");
  if (params.get("theme")) config.theme = params.get("theme");
  const sim = { cpu: 18, mem: 58, gpu: 22, down: 600_000, up: 80_000 };
  let metrics = mockMetrics(sim, config, forced);
  setInterval(() => {
    metrics = mockMetrics(sim, config, forced);
    window.dispatchEvent(new CustomEvent("app-metrics", { detail: metrics }));
  }, 1000);

  const emitConfig = () => window.dispatchEvent(new CustomEvent("app-config", { detail: clone(config) }));

  return {
    isTauri: false,
    getConfig: async () => clone(config),
    saveConfig: async (next) => {
      config = clone(next);
      emitConfig();
      return clone(config);
    },
    resetConfig: async () => {
      config = clone(fallbackConfig);
      emitConfig();
      return clone(config);
    },
    getMetrics: async () => metrics,
    getAutostart: async () => config.autostart,
    getPlatform: async () => "windows",
    getVersion: async () => "1.2.0",
    setAutostart: async (enabled) => {
      config.autostart = enabled;
      return enabled;
    },
    showSettings: async () => {},
    resetFloatingPosition: async () => clone(config),
    persistFloatingPosition: async () => clone(config),
    fitFloating: async () => {},
    showFloatingMenu: async () => {},
    showLogFolder: async () => "C:\\Users\\you\\AppData\\Local\\PulseRing\\logs",
    quit: async () => {},
    listen: async () => () => {},
    onMoved: async () => () => {},
    startDragging: async () => {}
  };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function walk(value, min, max, step) {
  return Math.min(max, Math.max(min, value + (Math.random() - 0.5) * step));
}

function classify(value, [warn, critical]) {
  if (value >= critical) return "high";
  if (value >= warn) return "medium";
  return "normal";
}

function mockMetrics(sim, config, forced) {
  sim.cpu = walk(sim.cpu, 3, 100, 22);
  sim.mem = walk(sim.mem, 35, 97, 4);
  sim.gpu = walk(sim.gpu, 0, 100, 18);
  sim.down = walk(sim.down, 0, 60_000_000, 2_500_000);
  sim.up = walk(sim.up, 0, 4_000_000, 300_000);
  if (forced === "medium") Object.assign(sim, { cpu: Math.max(sim.cpu, 76) });
  if (forced === "high") Object.assign(sim, { mem: Math.max(sim.mem, 94) });

  const thresholds = THRESHOLDS[config.alert] || THRESHOLDS.standard;
  const levels = {
    cpu: classify(sim.cpu, thresholds.cpu),
    memory: classify(sim.mem, thresholds.memory),
    gpu: classify(sim.gpu, thresholds.gpu)
  };
  const order = ["normal", "medium", "high"];
  const pressure = order[Math.max(...Object.values(levels).map((level) => order.indexOf(level)))];
  const focus = pressure === "normal" || levels.cpu === pressure
    ? sim.cpu
    : levels.memory === pressure ? sim.mem : sim.gpu;
  const total = 32 * 1024 ** 3;
  return {
    cpu_percent: sim.cpu,
    memory: { used_bytes: total * sim.mem / 100, total_bytes: total, percent: sim.mem },
    network: { download_bps: sim.down, upload_bps: sim.up },
    gpu: {
      name: "NVIDIA GeForce RTX 4070 Laptop GPU",
      usage_percent: sim.gpu,
      memory_used_bytes: 2.8 * 1024 ** 3,
      memory_total_bytes: 8 * 1024 ** 3,
      temperature_celsius: 54,
      available: true,
      source: "nvml"
    },
    levels,
    pressure,
    focus_percent: focus,
    compact_text: ""
  };
}
