# Release Guide

This document is for maintainers who need to cut a release without reconstructing the project context from chat history.

## Version Source

The release version is defined in two places:

- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

Keep those values in sync before tagging a release.

## Tag Convention

Use a semantic version tag with a leading `v`, for example:

- `v1.0.0`
- `v1.0.1`
- `v1.1.0`

The GitHub Actions release workflow triggers on `v*` tags.

## What the Release Workflow Does

The workflow:

- Builds Windows installers and the portable executable.
- Builds the macOS DMG directly on macOS.
- Verifies the generated outputs.
- Uploads the artifacts to the workflow run.
- Publishes the same artifacts to the GitHub Release attached to the tag.

## Release Checklist

1. Bump the version in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Update `README.md` and `README.zh-CN.md` if they mention exact asset names.
3. Update `CHANGELOG.md`.
4. Commit the version bump.
5. Tag the commit, for example `git tag v1.0.1`.
6. Push the tag with `git push origin v1.0.1`.
7. Confirm the GitHub Actions release workflow completed successfully.

## Asset Expectations

Windows:

- `PulseRing_*_x64-setup.exe`
- `PulseRing_*_x64_en-US.msi`
- `PulseRing_*_x64-portable.exe`

macOS:

- `PulseRing_*.dmg`

The release process should never upload a renamed archive pretending to be a DMG.
