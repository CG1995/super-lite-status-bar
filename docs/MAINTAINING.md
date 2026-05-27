# Maintaining PulseRing

This document describes how to keep the project coherent as it grows.

## Product Boundary

PulseRing is a quiet desktop status monitor, not a full observability suite.

Good additions:

- Better tray or menu bar reliability.
- Lower idle resource usage.
- Clearer settings for existing behavior.
- More robust packaging, signing and release automation.
- Lightweight GPU support that degrades gracefully.

Changes to treat carefully:

- Large dashboard windows.
- Always-visible long text in the Windows tray.
- Background network services.
- Heavy frontend frameworks.
- Platform-specific behavior without a fallback.

## Repository Hygiene

- Keep generated binaries, installers, logs and local config out of git.
- Keep version numbers synchronized between `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
- Keep README download names aligned with the latest public release.
- Keep workflow changes small and easy to audit.

## Review Checklist

For code changes:

- Does it preserve best-effort behavior when metrics are unavailable?
- Does it avoid panics in background sampling paths?
- Does it keep the settings model synchronized between Rust and JavaScript?
- Does it affect Windows tray, floating window or macOS menu bar behavior?

For release changes:

- Does CI still run formatting, linting and tests?
- Does the release workflow upload only real Tauri artifacts?
- Does the macOS DMG come from the macOS build job?
- Are unsigned artifact warnings still accurate?

## Versioning

Use semantic versioning:

- Patch: bug fixes, packaging fixes, small UI polish.
- Minor: new user-facing options or platform support.
- Major: incompatible configuration or behavior changes.
