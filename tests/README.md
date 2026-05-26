# Test Plan / 测试计划

Rust unit tests live beside the modules in `src-tauri/src/core` so `cargo test` can run them directly.

Rust 单元测试放在 `src-tauri/src/core` 对应模块旁边，可直接通过 `cargo test` 运行。

## Covered Now / 当前已覆盖

- Config read/write and sanitization: `core/config.rs`
- Network speed delta calculation and counter reset handling: `core/network_speed.rs`
- Metric formatting: `core/system_metrics.rs`
- NVIDIA `nvidia-smi` output parsing: `core/gpu.rs`, Windows only

## Manual Checks / 当前建议人工验证

- Windows tray hover popup appears when the pointer enters the tray icon.
- Popup hides when the pointer leaves the tray icon.
- Right-click tray menu is not blocked by the popup.
- Settings window opens from the tray menu.
- Autostart switch reads and writes real system state.
- Windows mini floating bar can be enabled and closed.
- Quit exits the background process.

## Planned / 后续计划

- Single-instance behavior test.
- Autostart state integration test.
- Windows Explorer restart tray recovery test.
- High-DPI screenshot test.
- Suspend/resume sampling recovery test.
- macOS menu bar behavior test.
