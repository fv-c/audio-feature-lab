# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: workspace scaffold, validated config system, domain/JSON output model, filesystem walker with file identity baseline, native Essentia boundary scaffold, a streaming core pipeline, append-friendly JSONL storage, and a sober CLI surface.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point with analyze, batch, scan, backend-info, schema, and config commands
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain model, JSON output model, walker/file identity layer, streaming pipeline, and JSONL writer
- `crates/afl-essentia-sys`: raw C FFI declarations for the native wrapper
- `crates/afl-essentia`: safe Rust wrapper skeleton around the native boundary
- `crates/audio-feature-lab-ffi`: project-facing façade for the preferred backend API
- `native/essentia-wrapper`: C++ wrapper scaffold and CMake project for local Essentia integration
- `configs/*.toml`: minimal, default, and research example profile files
- `docs/agent`: authoritative project constraints and execution plan

## Scope

The repository now includes the workspace scaffold, typed profile configuration, a deterministic JSON record model for analysis output, a recursive filesystem walker with metadata-based file identity, a minimal native boundary scaffold that keeps the Essentia interface at a JSON-string-per-file contract, a streaming pipeline that processes files one by one into JSONL-ready records, an append-friendly JSONL storage layer with line-by-line validation helpers, and a sober CLI that exposes single-file analysis, batch analysis, dry-run scanning, backend inspection, schema inspection, and config validation. Full local Essentia integration is still intentionally unimplemented and explicitly marked as environment-dependent.
