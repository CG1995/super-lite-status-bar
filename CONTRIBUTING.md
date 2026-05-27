# Contributing

Thanks for taking the time to improve PulseRing.

This project is intentionally small. Changes should keep the app quiet, lightweight and easy to reason about.

## Development Setup

Prerequisites:

- Rust stable
- Tauri CLI v2
- Platform build tools:
  - Windows: Visual Studio 2022 Build Tools with MSVC C++ tools and WebView2 Runtime
  - macOS: Xcode command line tools

Run the app from the Tauri package:

```bash
cd src-tauri
cargo run
```

## Checks

Run these before opening a pull request:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```

## Project Principles

- Keep Windows tray behavior compact and platform-native.
- Do not put long status text directly in the Windows tray.
- Keep GPU collection best-effort and non-fatal.
- Avoid adding heavyweight frontend frameworks unless there is a clear maintenance win.
- Prefer small, focused pull requests over broad rewrites.

## Pull Request Expectations

Please include:

- What changed.
- Why the change is needed.
- How it was tested.
- Any platform-specific behavior, especially Windows tray or macOS menu bar behavior.

## Release Changes

Version bumps and release workflow changes should follow [docs/RELEASE.md](./docs/RELEASE.md).
