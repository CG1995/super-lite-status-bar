import { pressureClass, shortSpeed } from "../components/format.js";

export function renderFloatingBar(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  let dragTimer = null;
  document.documentElement.classList.add("floating-window");
  root.className = "floating-shell";
  root.innerHTML = `
    <section class="floating-wrap" data-floating-wrap>
      <section class="floating" data-floating></section>
      <button class="floating-lock" type="button" data-floating-lock aria-label="Pin floating bar"></button>
    </section>
  `;

  const wrap = root.querySelector("[data-floating-wrap]");
  const bar = root.querySelector("[data-floating]");
  const lockButton = root.querySelector("[data-floating-lock]");

  const syncControls = () => {
    const locked = Boolean(config.floating_bar.lock_position);
    wrap.classList.toggle("is-locked", locked);
    wrap.classList.toggle("is-click-through", locked && Boolean(config.floating_bar.click_through));
    lockButton.dataset.locked = String(locked);
    lockButton.setAttribute("aria-pressed", String(locked));
  };

  const render = (snapshot) => {
    if (!snapshot) return;
    currentMetrics = snapshot;
    bar.className = `floating ${pressureClass(snapshot)}`;
    bar.style.opacity = String(config.floating_bar.opacity ?? 0.92);
    bar.innerHTML = floatingContent(snapshot);
    syncControls();
  };

  render(initialMetrics);
  window.addEventListener("app-metrics", (event) => render(event.detail));
  window.addEventListener("app-config", (event) => {
    config = event.detail;
    render(currentMetrics);
  });

  wrap.addEventListener("mousedown", async (event) => {
    if (event.target.closest("[data-floating-lock]")) return;
    if (!config.floating_bar.lock_position && event.button === 0) {
      await api.startDragging();
      clearTimeout(dragTimer);
      dragTimer = setTimeout(async () => {
        config = await api.persistFloatingPosition();
      }, 250);
    }
  });

  lockButton.addEventListener("click", async (event) => {
    event.stopPropagation();
    config = await updateFloatingConfig(api, config, {
      lock_position: !config.floating_bar.lock_position
    });
    render(currentMetrics);
  });

  wrap.addEventListener("contextmenu", (event) => {
    event.preventDefault();
  });
}

async function updateFloatingConfig(api, config, patch) {
  const current = await api.persistFloatingPosition();
  const next = JSON.parse(JSON.stringify(current));
  Object.assign(next.floating_bar, patch);
  return api.saveConfig(next);
}

function floatingContent(snapshot) {
  return `
    ${item("CPU", `${Math.round(snapshot.cpu_percent)}%`)}
    ${item("MEM", `${Math.round(snapshot.memory.percent)}%`)}
    ${item("GPU", gpuValue(snapshot))}
    ${item("↓", shortSpeed(snapshot.network.download_bps, "auto"))}
    ${item("↑", shortSpeed(snapshot.network.upload_bps, "auto"))}
  `;
}

function item(label, value) {
  return `<span><b>${label}</b> ${value}</span>`;
}

function gpuValue(snapshot) {
  if (snapshot.gpu?.usage_percent != null) return `${Math.round(snapshot.gpu.usage_percent)}%`;
  return "N/A";
}
