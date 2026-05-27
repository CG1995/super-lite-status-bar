# Changelog / 变更记录

## [1.0.1] - 2026-05-27

### English

#### Added
- macOS GPU: VRAM parsing and GPU usage via `ioreg IOAccelerator` (was name-only).
- Windows AMD/Intel GPU: fallback via WMI (`Win32_VideoController`) when `nvidia-smi` unavailable.
- macOS floating bar: CoreGraphics cursor tracking (was Windows-only).
- Tray ring icon now changes color: blue = normal, orange = >=85% load, red = memory >=95%.
- CI: Windows portable exe + NSIS installer artifacts.
- CI: macOS DMG dual-architecture — x64 (Intel Mac) and ARM64 (Apple Silicon).

#### Changed
- Memory >=95% triggers red tray ring (visual alert, no popup notification).
- `pressure_level`: High reserved for memory >=95% only.

### 中文

#### 新增
- macOS GPU 完整支持：VRAM 解析 + GPU 使用率（`ioreg IOAccelerator`），之前仅获取名称。
- Windows AMD/Intel GPU 支持：nvidia-smi 不可用时回退到 WMI。
- macOS 浮动窗光标追踪：补齐 CoreGraphics 光标位置。
- 托盘环图标颜色变化：蓝=正常，橙=≥85%，红=仅内存≥95%。
- CI：Windows 便携版 + 安装包自动构建。
- CI：macOS DMG 双架构构建 — x64（Intel 老 Mac）和 ARM64（Apple Silicon 新 Mac）。

#### 变更
- 内存 ≥95% 托盘变红告警（不弹通知）。
- `pressure_level`：High 仅留给内存 ≥95%。

## [1.0.0] - 2026-05-26
