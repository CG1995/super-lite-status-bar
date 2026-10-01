/**
 * SVG ring gauge shared by every surface so the tray icon, tooltip, floating bar
 * and control center all speak the same visual language.
 */
export function ring({ percent = 0, level = "normal", size = 40, stroke = 5, className = "" } = {}) {
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const value = Number.isFinite(percent) ? Math.min(100, Math.max(0, percent)) : 0;
  // Keep a short visible arc at idle so the ring never reads as "empty / broken".
  const shown = Math.max(value, 4);
  const offset = circumference * (1 - shown / 100);
  const center = size / 2;
  return `
    <svg class="ring level-${level} ${className}" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" aria-hidden="true">
      <circle class="ring-track" cx="${center}" cy="${center}" r="${radius}" stroke-width="${stroke}" />
      <circle class="ring-value" cx="${center}" cy="${center}" r="${radius}" stroke-width="${stroke}"
        stroke-dasharray="${circumference.toFixed(2)}" stroke-dashoffset="${offset.toFixed(2)}"
        transform="rotate(-90 ${center} ${center})" />
    </svg>
  `;
}

/** Tiny line chart for recent history (values 0-100, or normalised by `max`). */
export function sparkline(values, { width = 120, height = 28, max = 100, className = "" } = {}) {
  if (!values.length) return `<svg class="spark ${className}" width="${width}" height="${height}"></svg>`;
  const top = Math.max(max, ...values, 1);
  const step = values.length > 1 ? width / (values.length - 1) : width;
  const points = values.map((value, index) => {
    const x = index * step;
    const y = height - 2 - (Math.max(0, value) / top) * (height - 4);
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const area = `0,${height} ${points.join(" ")} ${((values.length - 1) * step).toFixed(1)},${height}`;
  return `
    <svg class="spark ${className}" width="100%" height="${height}" viewBox="0 0 ${width} ${height}" preserveAspectRatio="none" aria-hidden="true">
      <polygon class="spark-area" points="${area}" />
      <polyline class="spark-line" points="${points.join(" ")}" />
    </svg>
  `;
}
