# Third-Party Software

## Essentia

- Role: backend for audio descriptor extraction
- Status: integrated behind the optional `native-backend` feature
- Upstream site: https://essentia.upf.edu/
- Integration model: local native dependency discovered through `ESSENTIA_PREFIX` and `pkg-config`, wrapped by `native/essentia-wrapper`
- Licensing note: this repository is AGPL-oriented because Essentia is the intended analysis backend

## Other notable build and runtime dependencies

- Rust crates from crates.io, including `serde`, `serde_json`, `toml`, `criterion`, `cmake`, and `pkg-config`
- platform C++ runtime libraries required by the native wrapper when `native-backend` is enabled

This file is an engineering inventory, not a complete legal notice set. See [docs/licensing.md](docs/licensing.md) for the repository-level licensing stance.
