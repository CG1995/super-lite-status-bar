# Changelog / 变更记录

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
- 覆盖配置、网络速度、GPU 解析、指标格式化等单元测试。

#### 已知限制

- Windows 产物尚未签名。
- macOS 菜单栏行为仍需真机验证。
- GPU 支持目前主要覆盖 Windows NVIDIA。
