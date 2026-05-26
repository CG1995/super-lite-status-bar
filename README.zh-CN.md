# 脉环

脉环是一个基于 Tauri 2 的轻量桌面状态工具，面向 Windows 10/11 和 macOS。它常驻托盘或菜单栏，用一个安静的小入口显示 CPU、内存、显卡和网络状态，不打扰，也不抢屏幕。

English name: PulseRing  
English documentation: [README.md](./README.md)

## 下载

Windows 构建产物发布在 [GitHub Releases 页面](https://github.com/CG1995/super-lite-status-bar/releases/latest)。

- 推荐下载：`PulseRing_1.0.0_x64-setup.exe`
- 备用安装包：`PulseRing_1.0.0_x64_en-US.msi`
- 免安装可执行文件：`super-lite-status-bar.exe`

当前 Windows 产物尚未做代码签名，首次运行时 Windows 可能会出现 SmartScreen 提示。

## 监控指标

- CPU 总使用率
- 内存使用量、总量、百分比
- 网络下载 / 上传速度
- GPU 使用率、显存占用、温度和型号，能获取多少显示多少

GPU 采用能力检测。当前平台或硬件无法获取 GPU 数据时，应用不会崩溃，会显示 N/A 或降级展示。

## 当前交互

### Windows

- 托盘只显示图标，不在 Windows 托盘里塞长文本。
- 鼠标悬停托盘图标时显示四行状态弹窗：CPU、内存、GPU、网络。
- 右键菜单包含：设置、开机自启动、悬浮窗、日志、退出。
- 悬浮窗可在设置里开启或关闭。
- 悬浮窗只保留悬停出现的 pin 固定按钮；透明度、穿透、置顶等选项统一放在正式设置页。

### macOS

- Tauri 菜单栏能力已预留。
- 短文本菜单栏模式仍需 macOS 真机测试和完善。

## 技术栈

- Tauri 2
- Rust 后端
- 极简无框架前端：HTML、CSS、JavaScript
- `sysinfo` 采集 CPU、内存、网络计数器
- Windows 上通过 `nvidia-smi` 尝试采集 NVIDIA GPU 指标
- Tauri autostart 插件
- Tauri single-instance 插件

## 构建

Windows 依赖：

- Rust stable 工具链
- Microsoft Visual Studio 2022 Build Tools，包含 MSVC C++ 工具
- WebView2 Runtime

运行：

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo run
```

测试：

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo test
```

打包：

```powershell
cd C:\path\to\super-lite-status-bar\src-tauri
cargo tauri build --bundles nsis msi --no-sign --ci
```

Windows 打包产物：

```text
src-tauri/target/release/bundle/nsis/PulseRing_1.0.0_x64-setup.exe
src-tauri/target/release/bundle/msi/PulseRing_1.0.0_x64_en-US.msi
```

## 安全与隐私

- 不要提交个人 token、GitHub PAT、日志、本地配置文件。
- 用户配置保存到系统用户配置目录。
- 日志写入系统对应的应用日志目录。
- GPU 采集必须保持 best-effort，失败不能影响主流程。

## 当前状态

详见 [docs/DEVELOPMENT_PROGRESS.md](./docs/DEVELOPMENT_PROGRESS.md) 和 [CHANGELOG.md](./CHANGELOG.md)。

## License

项目 License 尚未由项目所有者最终确认。
