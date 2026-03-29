# Architecture

This document describes the current implementation in the preserved workspace layout.

## Design Summary

`audio-feature-lab` is split so that Rust owns orchestration and storage, while Essentia owns descriptor extraction. The design goal is to keep the native boundary narrow, keep memory bounded in batch runs, and preserve a stable JSONL schema regardless of backend internals.

## Workspace Roles

- `crates/audio-feature-lab-cli`
  Handles command parsing, help text, command dispatch, progress reporting, and batch summaries.
- `crates/audio-feature-lab-config`
  Loads TOML configuration, validates built-in profiles, validates feature families and names, and enforces the current aggregation and frame-level constraints.
- `crates/audio-feature-lab-core`
  Owns the domain model, walker, file identity extraction, streaming pipeline orchestration, and JSONL storage primitives.
- `crates/audio-feature-lab-ffi`
  Provides the project-facing backend API used by the rest of the Rust workspace.
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
- repository-level provenance fields

Essentia is responsible for:

- descriptor extraction
- frame-level and aggregated feature computation
- backend-specific analysis failures

## Streaming And Concurrency Strategy

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

Current semantic split:

- `features` contains the available file-level descriptor values for the analyzed file
- `aggregation` contains statistic-labeled exports of those descriptors
- with the current Essentia backend, `mean` is the only supported aggregation statistic, so `features` and `aggregation.*.*.mean` often carry the same underlying numeric content

## Failure Model

The pipeline tries to preserve one record per requested file, even when analysis fails:

- backend call failures become records with an error `status`
- backend payload parse failures become records with an error `status`
- file-level failures are surfaced on `stderr` in batch mode and remain visible in JSONL output
- unsupported requested descriptors are omitted and reported through warnings rather than being renamed or approximated

This is intentional. The tool is meant for corpus work, where partial completion with explicit failures is usually more useful than an aborted whole-run result.

## Backend Surface

The only supported public backend today is Essentia:

- config uses `[backend].name = "essentia"`
- the façade crate exists so the rest of the workspace does not depend directly on raw native details
- no alternative backend is exposed publicly at the moment

## Current Known Gaps

- the `schema` block exists but is still reserved rather than fully populated
- the `file` block currently uses baseline identity fields instead of canonical paths and content hashes
- persistent cross-run caching is not implemented yet; today the repository has file identity primitives and skip-logic benchmarking groundwork, not a finished cache layer
- the config and model accept the full allowed aggregation vocabulary, but the current native Essentia path only supports `mean`
- the current native backend is validated locally on macOS; Linux and Windows native validation remain open work
- the current backend omits some requested descriptors with warnings instead of approximating them
