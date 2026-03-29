# Platforms

The repository is designed for macOS, Windows, and Linux, but the level of completion differs between the Rust-only path and the native Essentia path.

## Current Support Summary

- Rust-only workspace: intended and CI-covered on macOS, Windows, and Linux
- native Essentia backend: validated locally on macOS only
- macOS universal packaging: planned, not yet automated

## Shared Rust Code

The shared Rust code is written to remain platform-neutral where practical:

- path handling uses `Path` and `PathBuf`
- walker logic avoids Unix-only shortcuts in shared code
- CLI behavior is consistent across platforms where the backend is available
- JSONL storage is independent of platform-specific file formats

## macOS

Current status:

- the native backend has been exercised locally on macOS
- the build script links the C++ wrapper and `Accelerate`
- local development currently assumes an Essentia installation reachable through `ESSENTIA_PREFIX` or `/tmp/essentia-install`
- the new backend-aware Rust path also knows about `mpeg7`, but there is no macOS MPEG-7 native wrapper linked yet

Universal-binary strategy:

- build the native dependency stack for `arm64` and `x86_64`
- build matching Rust artifacts for both architectures
- merge distributable binaries only after both native dependency sets are available

This strategy is not yet automated in CI or release packaging.

## Linux

Current status:

- the Rust workspace is expected to build and test normally without `native-backend`
- the native build path is designed around `pkg-config`, CMake, and `libstdc++`
- repository automation does not yet validate a real Linux Essentia installation

Current Linux native TODOs:

- validate the build against a real packaged or locally built Essentia install
- document required system packages more precisely once a reference environment is fixed
- add a native smoke job only after the environment can be reproduced reliably
- choose and integrate a real Linux-side MPEG-7 backend implementation before claiming Linux MPEG-7 support

## Windows

Current status:

- the Rust workspace is part of the intended support surface
- the default CI workflow includes Windows for `fmt`, `clippy`, and `test`
- the native Essentia backend is not yet validated on Windows

Current Windows native TODOs:

- define the supported compiler and CMake toolchain combination
- document how Essentia and its dependencies are built or installed on Windows
- add Windows-specific linker/runtime handling in the native build path where required
- revisit the UTF-8-only `const char*` path boundary if Windows path behavior demands a different representation
- choose and integrate a real Windows-side MPEG-7 backend implementation before claiming Windows MPEG-7 support

## CI And Release Realism

The default GitHub Actions workflow is target-aware for:

- macOS
- Windows
- Linux

But it intentionally exercises the Rust-only workspace. Native Essentia jobs are not added yet because the dependency setup is still environment-sensitive and not reproducible enough for generic CI.

## Packaging Expectations

What exists now:

- a normal Rust workspace build
- an optional native backend feature
- a documented local native build path

What remains open:

- universal macOS release automation
- Linux native packaging guidance
- Windows native packaging guidance
- release artifacts that bundle or document native dependencies per platform
