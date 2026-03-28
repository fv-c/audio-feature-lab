# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: Phase 1 repository scaffold only.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point and CLI skeleton
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain and pipeline skeleton
- `crates/audio-feature-lab-ffi`: minimal FFI boundary placeholder
- `configs/*.toml`: minimal, default, and research example profile files
- `docs/agent`: authoritative project constraints and execution plan

## Scope

This phase establishes the workspace, crate boundaries, and profile/config layout without implementing audio analysis business logic yet.
