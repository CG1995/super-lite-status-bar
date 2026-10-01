# PulseRing

<p align="center">
  <img src="./src-tauri/icons/128x128.png" alt="PulseRing app icon" width="160">
</p>

<p align="center">
  <a href="https://github.com/CG1995/super-lite-status-bar/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/CG1995/super-lite-status-bar/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/CG1995/super-lite-status-bar/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/CG1995/super-lite-status-bar?display_name=tag"></a>
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24c8db">
  <img alt="Platforms" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-555">
</p>

PulseRing is a quiet Tauri 2 desktop utility for Windows 10/11 and macOS. It lives in the system tray or menu bar and gives you a quick read on CPU, memory, GPU and network activity without opening a full monitoring dashboard.

中文名：脉环  
中文文档: [README.zh-CN.md](./README.zh-CN.md)

## Download

Windows builds are published on the [GitHub Releases page](https://github.com/CG1995/super-lite-status-bar/releases/latest).

- Recommended: `PulseRing_1.2.0_x64-setup.exe`
- Alternative installer: `PulseRing_1.2.0_x64_en-US.msi`
- Portable executable: `PulseRing_1.2.0_x64-portable.exe`

macOS builds are published on the same Releases page.

- Recommended: `PulseRing_1.2.0_aarch64.dmg`
- The DMG is produced by Tauri on macOS, so it is a real disk image rather than a renamed archive.

The current Windows artifacts are unsigned, so Windows may show a SmartScreen warning on first launch.
The current macOS artifacts are unsigned, so macOS may still prompt the first time you open the app.

Releases are tag-driven. The GitHub Actions release workflow builds Windows and macOS bundles from the same tag and uploads the generated assets directly.

## Preview

<p align="center">
  <img src="./docs/assets/control-center-overview.jpg" alt="Control center overview with live status rings" width="640">
</p>

<p align="center">
  <img src="./docs/assets/tray-tooltip.jpg" alt="Tray hover card" width="300">
  <img src="./docs/assets/control-center-floating.jpg" alt="Floating bar settings with live preview" width="420">
</p>

## What It Monitors

- CPU total usage
- Memory usage, used / total / percentage
- Network download and upload speed
- GPU usage, VRAM usage, temperature and model when available

GPU metrics are capability-based. The app degrades gracefully when GPU data is unavailable.

## Current UX

### Status colors

Every surface shares one status palette: **green** (normal), **amber** (elevated) and **red** (critical).
Each metric (CPU, memory, GPU) gets its own level; the overall level is the hottest of the three.
Thresholds come from the alert sensitivity setting (relaxed / standard / sensitive), and a level only
steps back down after the value drops about 4 points below the threshold, so colors do not flicker.

### Windows

- **Tray icon** is a ring gauge: its color is the overall level and its arc shows the load driving it.
- **Hover** the tray icon for a status card (overall state, CPU, memory, GPU with meters, network).
- **Left-click** opens the control center; **right-click** opens the menu (floating bar, lock, click-through, autostart, logs, quit).
- **Floating bar**: auto-sized capsule; opacity affects only the background so text stays crisp even at 0%.
  Drag to move, double-click for the control center, right-click for the menu, and click the grip/lock button to lock.
  With click-through enabled (requires lock), only the lock button stays clickable.
- **Control center**: live overview with history sparklines, floating-bar settings with a live preview over a wallpaper,
  alert sensitivity with threshold table, refresh rate, theme (system / light / dark) and autostart.

### macOS

- Tauri menu bar support is scaffolded.
- Short menu bar text still needs macOS device testing.

## Tech Stack

- Tauri 2
- Rust backend
- Minimal no-framework frontend: HTML, CSS, JavaScript
- `sysinfo` for CPU, memory and network counters
- In-process NVML dynamic FFI for discrete NVIDIA GPUs with automatic fallback to DXGI + PDH for integrated graphics (Intel Arc, AMD, Intel UHD) on Windows
- Tauri autostart plugin
- Tauri single-instance plugin

## Project Structure

```text
src-tauri/
  src/
    core/
    ui/
  tauri.conf.json
ui/
  components/
  floating_bar/
  settings/
  tray/
docs/
scripts/
tests/
```

## Build

Windows prerequisites:

- Rust stable toolchain
- Microsoft Visual Studio 2022 Build Tools with MSVC C++ tools ("Desktop development with C++" workload and a Windows SDK)
- WebView2 Runtime

Build from PowerShell or a Developer prompt. Git Bash ships its own `link` command, which shadows the MSVC linker.

Run:

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo run
```

Test:

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo test
```

Preview the UI without the Tauri shell (uses a mock data source; add `?level=high` or `?level=medium` to force a state):

```powershell
python -m http.server 5179 --directory ui
# open http://localhost:5179/index.html#settings  (or #tooltip, #floating)
```

Package:

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo tauri build --bundles nsis msi --no-sign --ci
```

Windows packaging produces:

```text
src-tauri/target/release/bundle/nsis/PulseRing_1.2.0_x64-setup.exe
src-tauri/target/release/bundle/msi/PulseRing_1.2.0_x64_en-US.msi
src-tauri/target/release/PulseRing_1.2.0_x64-portable.exe
```

macOS packaging produces:

```bash
cd /path/to/super-lite-status-bar/src-tauri
cargo tauri build --bundles dmg --no-sign --ci
```

```text
src-tauri/target/release/bundle/dmg/PulseRing_1.2.0_aarch64.dmg
```

## Release

This repository uses a tag-based release flow.

- CI runs `cargo fmt --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, and `cargo test --locked` on both Windows and macOS.
- The release workflow publishes Windows installers and the macOS DMG from a single tag push such as `v1.0.0`.
- Before cutting a release, bump the version in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, then update the asset names in this README if needed.

Release details live in [docs/RELEASE.md](./docs/RELEASE.md).

## Security And Privacy

- Do not commit personal tokens, GitHub PATs, logs or local config files.
- User configuration is stored under the OS user config directory.
- Logs are written to the OS-specific app log directory.
- GPU collection must remain best-effort and non-fatal.

## Current Status

See [docs/DEVELOPMENT_PROGRESS.md](./docs/DEVELOPMENT_PROGRESS.md) and [CHANGELOG.md](./CHANGELOG.md).

## Contributing

Focused issues and pull requests are welcome. Start with [CONTRIBUTING.md](./CONTRIBUTING.md), [SUPPORT.md](./SUPPORT.md), and [docs/MAINTAINING.md](./docs/MAINTAINING.md).

Security-sensitive reports should follow [SECURITY.md](./SECURITY.md).

## License

License has not been finalized by the project owner.
