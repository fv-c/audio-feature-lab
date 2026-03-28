# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: workspace scaffold, validated config system, domain/JSON output model, filesystem walker with file identity baseline, and a minimal native Essentia boundary scaffold.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point and CLI skeleton
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain model, JSON output model, and walker/file identity layer
- `crates/afl-essentia-sys`: raw C FFI declarations for the native wrapper
- `crates/afl-essentia`: safe Rust wrapper skeleton around the native boundary
- `crates/audio-feature-lab-ffi`: project-facing façade for the preferred backend API
- `native/essentia-wrapper`: C++ wrapper scaffold and CMake project for local Essentia integration
- `configs/*.toml`: minimal, default, and research example profile files
- `docs/agent`: authoritative project constraints and execution plan

## Scope

The repository now includes the workspace scaffold, typed profile configuration, a deterministic JSON record model for analysis output, a recursive filesystem walker with metadata-based file identity, and a minimal native boundary scaffold that keeps the Essentia interface at a JSON-string-per-file contract. Full local Essentia integration is still intentionally unimplemented and explicitly marked as environment-dependent.
