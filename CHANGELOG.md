# Changelog / 变更记录

## [1.0.3] - 2026-09-22

### English

#### Added
- Added support for integrated graphics (iGPU) telemetry on Windows (Intel Arc, Intel UHD/Iris, AMD Radeon) via native in-process DXGI (`dxgi.dll`) and Windows Performance Data Helper (`pdh.dll`).
- Integrated graphics displays memory usage percentage (e.g. `GPU 27%`) and detailed dedicated/shared video memory usage in tooltip without requiring external tools.
- Dual-tier GPU fallback architecture: seamlessly prioritizes discrete NVIDIA GPU when connected (via in-process NVML), and gracefully falls back to the integrated GPU when the NVIDIA GPU or eGPU is disconnected.
- Cleaned up GPU name display in tray tooltip by removing trademark noise `(R)`, `(TM)` and highlighting model names nicely (e.g. `Arc B390`).

### 中文

#### 新增
- Windows 下新增核心显卡/集成显卡（Intel Arc、Intel UHD/Iris、AMD Radeon 等核显）显存与使用率监控，通过进程内 DXGI (`dxgi.dll`) 与 Windows 原生性能计数器 (`pdh.dll`) 微秒级获取已用显存与总可用内存。
- 悬浮条与托盘在核显环境下正常显示显存占用率（如 `GPU 27%`）与显存容量（如 `4.9G / 18.0G`），不再显示 `N/A`。
- 实现双层显卡自适应回退架构：当插入外置 NVIDIA 独显（如 OCuLink eGPU）时，NVML 优先接管并展示独显核心利用率、专用显存与温度；断开外置显卡时自动平滑回退到核显，实现零闪退、零子进程与零弹窗。
- 优化托盘显卡名称净化逻辑，去除 `(R)`、`(TM)` 商标符号，优雅展示如 `Arc B390`、`RTX 3060 Ti`。

## [1.0.2] - 2026-09-22

### English

#### Fixed
- Fixed high CPU usage and process churn caused by continuous 1-second `nvidia-smi` CLI polling. NVIDIA GPU telemetry is now queried directly in-process via dynamic NVML (`nvml.dll`) FFI.
- Fixed Windows shutdown error modal popup (`nvidia-smi.exe - Application Error 0xc0000142`). Child process creation during session teardown is completely eliminated.
- Configured Windows error mode (`SetErrorMode`) to prevent any modal crash popups during logoff or system shutdown.
- Added console/system shutdown event handler (`SetConsoleCtrlHandler`) to immediately pause background telemetry upon shutdown signal.
- Added graceful 30-second backoff when NVIDIA GPU or external eGPU is disconnected or unavailable.
- macOS DMG packaging and unified dual-platform release workflow improvements.

### 中文

#### 修复
- 彻底解决每秒盲轮询 `nvidia-smi.exe` 导致的系统高频创建进程与 CPU/电量损耗。Windows 下重构为进程内动态加载 NVML（`nvml.dll` 原生 FFI）直接获取指标，进程创建次数彻底归零。
- 彻底根治 Windows 关机或重启时报 `nvidia-smi.exe - 应用程序无法正常启动 (0xc0000142)` 系统模态弹窗卡死关机的问题。
- Windows 启动时配置 `SetErrorMode` 关键错误静默模式，防止系统注销/关机时抛出模态错误框。
- 注册 `SetConsoleCtrlHandler` 关机/注销信号监听，收到关机事件瞬间立即中断后台指标轮询。
- 增加显卡脱机或无 NVIDIA GPU（如外置 OCuLink eGPU 断开或纯核显设备）时的 30 秒优雅退避机制，热插拔自动恢复。
- 完善 macOS DMG 打包与 Windows/macOS 统一 tag 自动化发版流。

## [1.0.0] - 2026-05-26

### English

First public Windows build under the name **PulseRing**.

#### Included

- Tauri 2 + Rust implementation.
- Windows tray-only status monitor with a transparent royal-blue ring icon.
- Compact tray hover popup for CPU, memory, GPU and network.
- CPU, memory, network upload/download sampling.
- Best-effort NVIDIA GPU metrics through `nvidia-smi`.
- Persistent local configuration with corruption backup fallback.
- Real autostart integration through the Tauri autostart plugin.
- Single-instance behavior through the Tauri single-instance plugin.
- Optional floating window with hover-only pin control.
- Floating-window options in the main settings panel: enabled, opacity, always on top, lock position, click-through and reset position.
- Settings synchronization through the shared persisted config and `config-updated` event.
- Log directory support.
- Windows NSIS setup executable and MSI installer packaging.
- Unified portable executable naming as `PulseRing_1.0.0_x64-portable.exe`.
- Unit tests for config, network speed, GPU parsing and metric formatting.

#### Known limitations

- Windows artifacts are unsigned.
- macOS menu bar behavior still needs real-device validation.
- GPU support is currently strongest for NVIDIA on Windows.

### 中文

以 **脉环** 为名发布的第一个 Windows 公开版本。

#### 包含

- Tauri 2 + Rust 实现。
- Windows 托盘常驻监控，使用透明背景的宝蓝色环形图标。
- 托盘悬停弹窗显示 CPU、内存、GPU、网络。
- CPU、内存、网络上传 / 下载采样。
- 通过 `nvidia-smi` 尝试获取 NVIDIA GPU 指标。
- 本地配置持久化，配置损坏时自动备份并恢复默认。
- 通过 Tauri autostart 插件实现真实开机自启动。
- 通过 Tauri single-instance 插件实现单实例。
- 可选悬浮窗，悬浮窗只保留悬停出现的 pin 固定按钮。
- 悬浮窗设置集中在正式设置页：开启、透明度、置顶、锁定位置、点击穿透、恢复默认位置。
- 设置通过同一份持久化配置和 `config-updated` 事件同步。
- 支持日志目录。
- 支持 Windows NSIS exe 安装器和 MSI 安装包打包。
- 统一免安装可执行文件命名为 `PulseRing_1.0.0_x64-portable.exe`。
- 覆盖配置、网络速度、GPU 解析、指标格式化等单元测试。

#### 已知限制

- Windows 产物尚未签名。
- macOS 菜单栏行为仍需真机验证。
- GPU 支持目前主要覆盖 Windows NVIDIA。
