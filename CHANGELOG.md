# Changelog / 变更记录

All notable project changes are documented here.

所有重要变更都会记录在这里。

## [0.1.1] - 2026-05-26

### English

#### Fixed

- Removed the duplicate tray menu action `Close floating window`; the checked `Windows mini floating bar` item now owns both open and close behavior.
- Kept the tray menu check states synchronized with persisted configuration after config changes.
- Fixed settings-window synchronization when autostart is toggled from the tray menu while the settings window is already open.

### 中文

#### 修复

- 移除右键菜单里重复的 `关闭悬浮窗` 项；现在只保留 `Windows mini 悬浮条` 勾选项来负责开启和关闭。
- 配置变化后会同步刷新右键菜单的勾选状态。
- 修复在设置窗口已打开时，从托盘右键菜单切换“开机自启动”后设置界面不同步的问题。

## [0.1.0] - 2026-05-26

### English

#### Added

- Initial Tauri 2 + Rust implementation.
- Windows tray-only status monitor with custom hover popup.
- CPU, memory, network upload and download sampling.
- Best-effort NVIDIA GPU metrics through `nvidia-smi`.
- Compact Windows hover popup with CPU / memory / GPU / network rows.
- Lightweight cursor watchdog for more reliable tray popup show/hide behavior.
- Right-click tray menu with settings, autostart, floating bar control, logs and quit.
- Optional Windows mini floating bar.
- Persistent local configuration with corruption backup fallback.
- Tauri autostart plugin integration.
- Tauri single-instance plugin integration.
- Log directory support.
- Windows NSIS setup executable and MSI installer packaging.
- Unit tests for config, network speed, GPU parsing and metric formatting.

#### Changed

- Removed the large status panel window from Windows UX.
- Simplified settings to only keep practical controls.
- Fixed refresh interval to 1 second.
- Forced all core indicators to remain enabled by default.
- Removed Windows tray text usage to avoid unreadable tray content.
- Shortened GPU display names for compact UI.

#### Known Limitations

- macOS menu bar text mode still needs real-device validation.
- GPU support is currently strongest for NVIDIA on Windows.
- Windows installer artifacts are available, but still unsigned and need clean-machine validation.
- Final production icon is not finalized.

### 中文

#### 新增

- 初始 Tauri 2 + Rust 实现。
- Windows 托盘常驻监控，使用自定义悬停状态弹窗。
- CPU、内存、网络上传 / 下载采样。
- 通过 `nvidia-smi` 尝试获取 NVIDIA GPU 指标。
- Windows 悬停弹窗显示 CPU / 内存 / GPU / 网络四行数据。
- 增加轻量级鼠标位置 watchdog，提高托盘弹窗出现 / 消失可靠性。
- 右键菜单支持设置、开机自启动、悬浮条控制、日志和退出。
- 可选 Windows mini 悬浮条。
- 本地配置持久化，配置损坏时自动备份并恢复默认。
- 接入 Tauri autostart 插件。
- 接入 Tauri single-instance 插件。
- 支持日志目录。
- 支持 Windows NSIS exe 安装器和 MSI 安装包打包。
- 覆盖配置、网络速度、GPU 解析、指标格式化等单元测试。

#### 变更

- 移除 Windows 上的大型系统状态面板窗口。
- 精简设置页，只保留实际有用的控制项。
- 刷新间隔固定为 1 秒。
- 默认强制显示所有核心指标。
- 不再在 Windows 托盘区域显示长文本，避免文字过小。
- 缩短 GPU 名称，适配紧凑 UI。

#### 已知限制

- macOS 菜单栏短文本模式仍需真机验证。
- GPU 支持目前主要覆盖 Windows NVIDIA。
- Windows 安装产物已可用，但仍未签名，正式发布前需要干净机器验证。
- 最终生产 logo 尚未定稿。
