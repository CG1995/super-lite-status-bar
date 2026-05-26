export function bytes(value) {
  if (!Number.isFinite(value) || value <= 0) return "0 MB";
  const gib = value / 1024 / 1024 / 1024;
  if (gib >= 1) return `${gib.toFixed(1)} GB`;
  return `${Math.round(value / 1024 / 1024)} MB`;
}

export function speed(value, unit = "auto") {
  if (!Number.isFinite(value) || value <= 0) return unit === "mb" ? "0.0 MB/s" : "0 KB/s";
  if (unit === "kb") return `${Math.round(value / 1024)} KB/s`;
  if (unit === "mb") return `${(value / 1024 / 1024).toFixed(1)} MB/s`;
  return value >= 1024 * 1024
    ? `${(value / 1024 / 1024).toFixed(1)} MB/s`
    : `${Math.round(value / 1024)} KB/s`;
}

export function shortSpeed(value, unit = "auto") {
  return speed(value, unit).replace(" MB/s", "M").replace(" KB/s", "K");
}

export function percent(value) {
  return Number.isFinite(value) ? `${Math.round(value)}%` : "N/A";
}

export function pressureClass(snapshot) {
  return `pressure-${snapshot?.pressure || "normal"}`;
}
