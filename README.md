# Super Lite Status Bar

Super Lite Status Bar is a lightweight Tauri 2 desktop utility for Windows 10/11 and macOS. It stays in the system tray / menu bar and shows core system status without opening a distracting main window.

中文文档: [README.zh-CN.md](./README.zh-CN.md)

## Download

Windows builds are published on the [GitHub Releases page](https://github.com/CG1995/super-lite-status-bar/releases/latest).

- Recommended: `Super-Lite-Status-Bar_0.1.2_x64-setup.exe`
- Alternative installer: `Super-Lite-Status-Bar_0.1.2_x64_en-US.msi`
- Portable executable: `super-lite-status-bar.exe`

The current Windows artifacts are unsigned, so Windows may show a SmartScreen warning on first launch.

## What It Monitors

- CPU total usage
- Memory usage, used / total / percentage
- Network download and upload speed
- GPU usage, VRAM usage, temperature and model when available

GPU metrics are capability-based. The app degrades gracefully when GPU data is unavailable.

## Current UX

### Windows

- Tray icon only, no long unreadable text in the Windows tray.
- Hovering the tray icon shows a compact four-row status popup: CPU, memory, GPU, network.
- The popup is controlled by tray hover state plus a lightweight cursor watchdog so it appears and disappears reliably.
- Right-click opens the tray menu: settings, autostart, Windows mini floating bar, logs, quit.
- Optional Windows mini floating bar can be enabled from settings.

### macOS

- Tauri menu bar support is scaffolded.
- Short menu bar text is still a follow-up item and needs macOS device testing.

## Tech Stack

- Tauri 2
- Rust backend
- Minimal no-framework frontend: HTML, CSS, JavaScript
- `sysinfo` for CPU, memory and network counters
- `nvidia-smi` capability path for NVIDIA GPU metrics on Windows
- Tauri autostart plugin
- Tauri single-instance plugin

## Project Structure

```text
src-tauri/
  src/
    core/
      autostart.rs
      config.rs
      gpu.rs
      logger.rs
      network_speed.rs
      system_metrics.rs
    ui/
      floating_bar.rs
      tray.rs
      windows.rs
    main.rs
  tauri.conf.json
ui/
  components/
  floating_bar/
  settings/
  tray/
  main.js
  styles.css
tests/
  README.md
docs/
  DEVELOPMENT_PROGRESS.md
scripts/
  generate-icons.ps1
```

## Build And Run

### Windows Prerequisites

- Rust stable toolchain
- Microsoft Visual Studio 2022 Build Tools with MSVC C++ tools
- WebView2 Runtime

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

Release build:

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo build --release
```

The built executable is:

```text
src-tauri/target/release/super-lite-status-bar.exe
```

## Packaging

Tauri bundling is configured in `src-tauri/tauri.conf.json`.

```powershell
cd src-tauri
cargo tauri build --bundles nsis msi --no-sign --ci
```

Windows packaging produces:

```text
src-tauri/target/release/bundle/nsis/Super Lite Status Bar_0.1.2_x64-setup.exe
src-tauri/target/release/bundle/msi/Super Lite Status Bar_0.1.2_x64_en-US.msi
```

Installer artifacts still need final clean-machine validation and code signing before a production release.

## Security And Privacy

- Do not commit personal tokens, GitHub PATs, logs or local config files.
- The app stores user configuration under the OS user config directory.
- Logs are written to the OS-specific app log directory.
- GPU collection must remain best-effort and non-fatal.

## Current Status

See [docs/DEVELOPMENT_PROGRESS.md](./docs/DEVELOPMENT_PROGRESS.md) and [CHANGELOG.md](./CHANGELOG.md).

## License

License has not been finalized by the project owner.
