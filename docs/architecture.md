# Architecture

This document describes the current implementation in the preserved workspace layout.

## Workspace Roles

- `crates/audio-feature-lab-cli`
  Handles command parsing, help text, command dispatch, progress reporting, and batch summaries.
- `crates/audio-feature-lab-config`
  Loads TOML configuration, validates built-in profiles, validates feature families and names, and enforces the current aggregation and frame-level constraints.
- `crates/audio-feature-lab-core`
  Owns the domain model, walker, file identity extraction, streaming pipeline orchestration, and JSONL storage primitives.
- `crates/audio-feature-lab-ffi`
  Provides the project-facing backend API used by the rest of the Rust workspace and dispatches by configured backend name.
- `crates/afl-essentia`
  Contains the safe Rust wrapper around the raw native interface.
- `crates/afl-essentia-sys`
  Contains the raw FFI declarations and native build script.
- `native/essentia-wrapper`
  Contains the C++ wrapper that links to Essentia and returns compact JSON payloads.

## End-To-End Flow

1. CLI loads and validates a config or selects the built-in default profile.
2. The walker discovers candidate files and extracts a baseline identity from metadata.
3. The pipeline serializes the validated config once into backend JSON.
4. The pipeline calls the selected backend once per file.
5. The backend returns one JSON payload containing `audio`, `features`, `aggregation`, and `status`.
6. Rust constructs the full top-level record by adding `schema`, `file`, `analysis`, and `provenance`.
7. Records are written immediately to a sink, typically JSONL.

## Separation Of Concerns

Rust is responsible for:

- filesystem traversal
- configuration and validation
- batching and bounded concurrency
- schema shape and deterministic serialization
- JSONL storage
- CLI behavior, progress, and user-visible errors
- backend selection and repository-level provenance fields

Essentia is responsible for:

- descriptor extraction
- frame-level and aggregated feature computation
- backend-specific analysis failures

## Memory And Concurrency Strategy

- no full-dataset accumulation is allowed in the normal batch path
- records are written progressively as they become ready
- bounded worker queues keep concurrency measurable and memory usage predictable
- output ordering is preserved even in the parallel path
- worker counts are configuration-driven and clamped to the machine's available parallelism

The current built-in configs deliberately keep `workers = 1` because the local measured native path regressed at higher counts.

## Schema Ownership

The Rust pipeline owns the outer record shape. The native backend currently only returns the blocks that are naturally backend-specific:

- `audio`
- `features`
- `aggregation`
- `status`

This keeps the FFI small while preserving a stable top-level JSONL schema in Rust.

## Backend Selection

The current configuration includes an explicit backend section:

- `[backend].name = "essentia"`
- `[backend].name = "mpeg7"`

Today:

- `essentia` is the real working native backend path
- `mpeg7` is wired through config, CLI, pipeline dispatch, and provenance, but still lacks a linked native implementation

## Current Known Gaps

- the `schema` block exists but is still reserved rather than fully populated
- the `file` block currently uses baseline identity fields instead of canonical paths and content hashes
- the current native backend is validated locally on macOS; Linux and Windows native validation remain open work
- the current backend omits some requested descriptors with warnings instead of approximating them
