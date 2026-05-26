# Development Progress / 开发进度

This document is written for future maintainers and AI coding agents that need to continue the project without full chat context.

本文档用于后续维护者或 vibecoding 工具在缺少完整对话上下文时接手项目。

## Product Direction

### English

Super Lite Status Bar is intentionally small and quiet. It should not become a full monitoring suite. The primary workflow is:

1. App starts silently.
2. Windows tray icon stays visible.
3. Hover over the icon to see CPU, memory, GPU and network details.
4. Right-click for settings and quit.
5. Optional Windows mini floating bar is available, but disabled by default.

Windows tray text must not be used for long status strings. It is unreadable and conflicts with the platform's UI model.

### 中文

Super Lite Status Bar 的定位是小、轻、安静。它不应该演变成复杂监控软件。核心流程是：

1. 应用静默启动。
2. Windows 托盘显示图标。
3. 鼠标悬停图标查看 CPU、内存、GPU、网络详情。
4. 右键打开设置和退出。
5. Windows mini 悬浮条可选开启，默认关闭。

Windows 托盘不要显示长状态文本。它不可读，也不符合平台交互习惯。

## Current Implementation

### Backend

- `core/system_metrics.rs`
  - Samples CPU, memory, network and GPU.
  - Formats compact/full/tooltip text.
  - Forces GPU sampling to degrade gracefully.
- `core/network_speed.rs`
  - Calculates upload/download speed from counter deltas.
  - Handles counter reset without negative speed.
- `core/gpu.rs`
  - Windows: tries `nvidia-smi`.
  - macOS: currently attempts GPU name discovery only.
- `core/config.rs`
  - Persists user config as JSON.
  - Sanitizes legacy or invalid config into the current simplified behavior.
- `core/autostart.rs`
  - Wraps Tauri autostart plugin.
- `core/logger.rs`
  - Initializes file logging.
- `ui/tray.rs`
  - Builds tray menu and dynamic icon.
  - Controls custom tooltip hover behavior.
  - Uses a 50ms cursor watchdog on Windows to compensate for unreliable tray enter/leave events.
  - Suppresses tooltip briefly when right-clicking so the context menu is not blocked.
- `ui/floating_bar.rs`
  - Applies Windows mini floating bar visibility, size, position and click-through.
- `ui/windows.rs`
  - Positions settings and tooltip windows.

### Frontend

- `ui/tray/tooltip.js`
  - Renders the four-row hover popup.
- `ui/settings/settings.js`
  - Simplified auto-save settings UI.
- `ui/floating_bar/floating.js`
  - Compact mini floating bar.
- `ui/components/state.js`
  - Tauri command bridge and local mock fallback.
- `ui/components/format.js`
  - Formatting helpers.

## Verified On Current Windows Machine

- `cargo test`: 7 tests pass.
- `cargo build --release`: succeeds.
- Release executable runs from:

```text
src-tauri/target/release/super-lite-status-bar.exe
```

Observed release process memory during local runs was roughly 35-46 MB working set.

## Open Work

### Must Do Before Public Release

- Finalize production icon / logo.
- Verify installer packaging on a clean Windows machine.
- Verify startup-on-login behavior after installation.
- Test suspend/resume and Explorer restart behavior.
- Test DPI scaling at 100%, 125%, 150%, 200%.
- Add real macOS testing and polish macOS menu bar behavior.

### Good Next Tasks

- Add a Windows UI automation smoke test for tray tooltip behavior if practical.
- Add GitHub Actions coverage for `cargo test` on Windows and macOS.
- Add binary signing plan for Windows and macOS.
- Add crash-safe config migration if config schema changes.
- Consider AMD / Intel GPU support only if it remains lightweight.

## Security Notes

- Do not commit tokens, GitHub PATs, local logs, local config, installer secrets or signing certificates.
- Do not put user-specific paths in documentation except examples.
- Keep `src-tauri/target/`, `src-tauri/gen/`, logs and temp files out of git.
- If a secret was ever pasted into chat or local notes, treat it as compromised and rotate it outside the repository.

## Handoff Notes For Vibecoding

When continuing this project:

1. Start by running `cargo test` in `src-tauri`.
2. Read `src-tauri/src/ui/tray.rs` before changing tray behavior.
3. Do not reintroduce a large status panel window on Windows.
4. Do not put long text directly in the Windows tray.
5. Keep default refresh interval at 1 second unless there is measured evidence to change it.
6. GPU failures must never crash the app.
7. Prefer small, targeted changes and verify with a release build on Windows.
