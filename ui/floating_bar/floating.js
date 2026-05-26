import { pressureClass, shortSpeed } from "../components/format.js";

export function renderFloatingBar(root, api, config, initialMetrics) {
  let currentMetrics = initialMetrics;
  let dragTimer = null;
  root.className = "floating-shell";
  root.innerHTML = `
    <section class="floating-wrap" data-floating-wrap>
      <section class="floating" data-floating></section>
      <button class="floating-lock" type="button" data-floating-lock aria-label="锁定悬浮条"></button>
      <section class="floating-menu" data-floating-menu hidden>
        <label class="floating-menu-row">
          <span>锁定</span>
          <input type="checkbox" data-floating-menu-lock />
        </label>
        <label class="floating-menu-row">
          <span>点击穿透</span>
          <input type="checkbox" data-floating-menu-click-through />
        </label>
        <label class="floating-menu-range">
          <span>透明度</span>
          <input type="range" min="0.35" max="1" step="0.05" data-floating-menu-opacity />
        </label>
      </section>
    </section>
  `;

  const wrap = root.querySelector("[data-floating-wrap]");
  const bar = root.querySelector("[data-floating]");
  const lockButton = root.querySelector("[data-floating-lock]");
  const menu = root.querySelector("[data-floating-menu]");
  const menuLock = root.querySelector("[data-floating-menu-lock]");
  const menuClickThrough = root.querySelector("[data-floating-menu-click-through]");
  const menuOpacity = root.querySelector("[data-floating-menu-opacity]");

  const syncControls = () => {
    const locked = Boolean(config.floating_bar.lock_position);
    wrap.classList.toggle("is-locked", locked);
    lockButton.dataset.locked = String(locked);
    lockButton.setAttribute("aria-pressed", String(locked));
    menuLock.checked = locked;
    menuClickThrough.checked = Boolean(config.floating_bar.click_through);
    menuOpacity.value = String(config.floating_bar.opacity ?? 0.92);
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
    if (event.target.closest("[data-floating-lock], [data-floating-menu]")) return;
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
    syncControls();
    menu.hidden = false;
  });

  window.addEventListener("click", (event) => {
    if (!event.target.closest("[data-floating-menu]")) {
      menu.hidden = true;
    }
  });

  menuLock.addEventListener("change", async () => {
    config = await updateFloatingConfig(api, config, { lock_position: menuLock.checked });
    render(currentMetrics);
  });

  menuClickThrough.addEventListener("change", async () => {
    config = await updateFloatingConfig(api, config, {
      click_through: menuClickThrough.checked
    });
    render(currentMetrics);
  });

  menuOpacity.addEventListener("input", () => {
    bar.style.opacity = String(menuOpacity.value);
  });

  menuOpacity.addEventListener("change", async () => {
    config = await updateFloatingConfig(api, config, {
      opacity: Number(menuOpacity.value)
    });
    render(currentMetrics);
  });
}

async function updateFloatingConfig(api, config, patch) {
  const next = JSON.parse(JSON.stringify(config));
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
