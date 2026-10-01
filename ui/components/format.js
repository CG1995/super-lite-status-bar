export const LEVEL_TEXT = {
  normal: "正常",
  medium: "偏高",
  high: "告急"
};

export const OVERALL_TEXT = {
  normal: "运行平稳",
  medium: "负载偏高",
  high: "负载过高"
};

// Mirrors AlertSensitivity::thresholds in src-tauri/src/core/config.rs.
export const THRESHOLDS = {
  relaxed: { cpu: [80, 95], memory: [88, 95], gpu: [85, 97] },
  standard: { cpu: [70, 90], memory: [80, 92], gpu: [75, 92] },
  sensitive: { cpu: [55, 80], memory: [70, 85], gpu: [60, 85] }
};

export function bytes(value) {
  if (!Number.isFinite(value) || value <= 0) return "0 MB";
  const gib = value / 1024 / 1024 / 1024;
  if (gib >= 1) return `${gib.toFixed(1)} GB`;
  return `${Math.round(value / 1024 / 1024)} MB`;
}

export function compactBytes(value) {
  return bytes(value).replace(" GB", "G").replace(" MB", "M");
}

export function speed(value) {
  if (!Number.isFinite(value) || value <= 0) return "0 KB/s";
  if (value >= 1024 * 1024 * 1024) return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB/s`;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB/s`;
  return `${Math.round(value / 1024)} KB/s`;
}

export function shortSpeed(value) {
  return speed(value).replace(" GB/s", "G").replace(" MB/s", "M").replace(" KB/s", "K");
}

export function percent(value) {
  return Number.isFinite(value) ? `${Math.round(value)}%` : "N/A";
}

export function levelOf(snapshot, key) {
  return snapshot?.levels?.[key] || "normal";
}

export function overallLevel(snapshot) {
  return snapshot?.pressure || "normal";
}

/** Human summary of what drives the current level, e.g. "内存占用 93%". */
export function overallDetail(snapshot) {
  if (!snapshot) return "正在采集数据…";
  const level = overallLevel(snapshot);
  if (level === "normal") return "所有指标均在正常范围";
  const hot = [
    ["cpu", "CPU 占用", snapshot.cpu_percent],
    ["memory", "内存占用", snapshot.memory?.percent],
    ["gpu", "GPU 占用", snapshot.gpu?.usage_percent]
  ].filter(([key]) => levelOf(snapshot, key) === level);
  return hot.map(([, label, value]) => `${label} ${percent(value)}`).join(" · ");
}

export function shortGpuName(name) {
  if (!name) return "";
  const cleaned = name
    .replace(/\((?:R|TM)\)/gi, "")
    .replace(/NVIDIA|GeForce|Laptop GPU|Graphics|\bGPU\b/gi, "")
    .replace(/\s+/g, " ")
    .trim();
  const modelPart = cleaned.split(/\s+/).find((part) => /\d/.test(part) && part.length >= 3);
  if (modelPart) {
    const prefix = cleaned.match(/\b(RTX|GTX|Arc|Radeon|RX|Iris|UHD|Quadro)\b/i)?.[1];
    return prefix && prefix.toLowerCase() !== modelPart.toLowerCase() ? `${prefix} ${modelPart}` : modelPart;
  }
  return cleaned;
}

export function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
