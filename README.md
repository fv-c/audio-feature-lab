# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: workspace scaffold, validated config system, domain/JSON output model, and filesystem walker with file identity baseline.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point and CLI skeleton
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain model, JSON output model, and walker/file identity layer
- `crates/audio-feature-lab-ffi`: minimal FFI boundary placeholder
- `configs/*.toml`: minimal, default, and research example profile files
- `docs/agent`: authoritative project constraints and execution plan

## Scope

The repository now includes the workspace scaffold, typed profile configuration, a deterministic JSON record model for analysis output, and a recursive filesystem walker with extension filtering and metadata-based file identity. Audio analysis execution logic is still intentionally unimplemented.
