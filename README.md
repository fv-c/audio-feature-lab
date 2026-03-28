# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: workspace scaffold, validated config system, and domain/JSON output model.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point and CLI skeleton
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain model and JSON output model skeleton
- `crates/audio-feature-lab-ffi`: minimal FFI boundary placeholder
- `configs/*.toml`: minimal, default, and research example profile files
- `docs/agent`: authoritative project constraints and execution plan

## Scope

The repository now includes the workspace scaffold, typed profile configuration, and a deterministic JSON record model for analysis output. Audio analysis execution logic is still intentionally unimplemented.
