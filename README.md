# audio-feature-lab

`audio-feature-lab` is a Rust command-line tool for large-scale audio feature extraction with Essentia as the analysis backend.

It is designed for corpus work:

- recursive folder scanning
- typed profile-driven configuration
- one-record-per-file JSONL output
- bounded-memory batch processing
- deterministic nested feature serialization
- explicit provenance and analysis status per file

The repository is written to behave like a real tool, not a notebook or ad hoc script. Unsupported descriptors are omitted with warnings instead of being renamed or approximated.

## Current Status

Today the only supported analysis backend is Essentia.

What works:

- CLI commands for analysis, batch processing, dry-run scanning, config validation, backend inspection, and schema explanation
- TOML configuration with built-in `minimal`, `default`, and `research` profiles
- recursive walker with extension filtering, hidden-file policy, symlink policy, and file identity extraction
- streaming JSONL writing with one record per analyzed file
- bounded worker-pool batch orchestration
- local benchmark suite for walker, pipeline, JSONL, and native runs

What remains incomplete:

- some requested descriptors are still omitted by the current Essentia wrapper
- the `schema` block exists but is still sparse
- canonical paths and content hashes are not yet in the `file` block
- persistent cross-run caching is not implemented yet
- native validation is still local on macOS; Linux and Windows native backend work remains open

## Requirements

Rust-only commands work with a normal Rust toolchain.

Real analysis also requires a local Essentia installation reachable by the build script:

- set `ESSENTIA_PREFIX=/path/to/essentia/prefix`, or
- provide the repository-local fallback at `/tmp/essentia-install`

The native wrapper is built through `native/essentia-wrapper/CMakeLists.txt`.

## Quick Start

Commands that do not require Essentia:

```sh
cargo run -p audio-feature-lab -- validate-config configs/default.toml
cargo run -p audio-feature-lab -- scan fixtures/audio --dry-run
cargo run -p audio-feature-lab -- schema
cargo run -p audio-feature-lab -- backend-info
```

Real analysis with a local Essentia installation:

```sh
export ESSENTIA_PREFIX=/path/to/essentia/prefix
cargo run -p audio-feature-lab --features native-backend -- analyze fixtures/audio/short-stereo-44k.wav
cargo run -p audio-feature-lab --features native-backend -- batch fixtures/audio --output results.jsonl
```

Batch progress and summaries are written to `stderr`. JSONL records go to `stdout` or to the file passed with `--output`.

## CLI

Supported commands:

- `analyze <file>`
- `batch <path>`
- `scan <path> --dry-run`
- `backend-info`
- `schema`
- `validate-config <file>`
- `profiles`
- `explain-schema`

Typical usage:

```sh
cargo run -p audio-feature-lab --features native-backend -- batch /path/to/corpus --config configs/default.toml --output results.jsonl
```

Operational notes:

- `analyze` writes one record
- `batch` accepts either a single file or a directory tree
- `scan --dry-run` only lists candidate files; it does not invoke Essentia
- batch progress lives on `stderr`, so redirecting `stdout` still produces clean JSONL
- analysis failures still produce per-file records with `status.success = false`
- a silent or otherwise backend-rejected file is reported in the batch summary and still kept as an error record in JSONL

## Configuration

Configuration is TOML and is validated before analysis starts.

The repository ships three built-in profiles:

- `configs/minimal.toml`
- `configs/default.toml`
- `configs/research.toml`

Current behavior:

- all shipped configs use `[backend].name = "essentia"`
- frame-level extraction is disabled by default
- shipped configs currently request `mean` aggregation only
- the config and data model accept the full allowed statistic set:
  - `mean`
  - `std`
  - `min`
  - `max`
  - `median`
  - `p10`
  - `p25`
  - `p75`
  - `p90`
- the current native Essentia backend still accepts only `mean`, so real analysis should keep `aggregation.statistics = ["mean"]` for now

See [docs/profiles.md](docs/profiles.md) for the profile-specific scope and caveats.

## Output

Primary storage format is JSONL: one JSON object per analyzed file.

Each record always contains these top-level blocks, in deterministic order:

1. `schema`
2. `file`
3. `audio`
4. `analysis`
5. `features`
6. `aggregation`
7. `provenance`
8. `status`

Important output rules:

- aggregation remains hierarchical as `aggregation.<family>.<feature>.<statistic>`
- vector-valued statistics remain arrays
- `features.<family>` now carries the available file-level descriptor values for the current file
- `features.frame_level` is always present
- when frame-level extraction is disabled, `features.frame_level` is `null`
- unsupported descriptors are omitted instead of being fabricated
- analysis failures are still represented as records through the `status` block

See [docs/feature-schema.md](docs/feature-schema.md) for the current record shape.

## Backend

Essentia is feature-gated so the Rust workspace can still build and test without a local native toolchain.

When `native-backend` is enabled:

- Rust handles orchestration, configuration, walker logic, schema ownership, JSONL writing, and user-facing errors
- Essentia handles descriptor extraction and backend-specific analysis work
- the FFI boundary remains one JSON-returning call per file
- the current native wrapper supports only `mean` aggregation statistics
- frame-level output is supported only for descriptors the backend can actually emit

Local build assumptions:

- `ESSENTIA_PREFIX` points to a usable Essentia installation
- the build script can also use `/tmp/essentia-install` as a repository-local fallback
- the native wrapper is built through `native/essentia-wrapper/CMakeLists.txt`

## Performance

The tool is engineered for streaming and bounded memory:

- no full-dataset accumulation in the batch path
- progressive sink writes
- config serialization once per pipeline instance
- backend version lookup cached once per pipeline instance
- bounded worker queues
- output ordering preserved in batch mode

Current local benchmark evidence is documented in [docs/performance.md](docs/performance.md). The built-in configs keep `workers = 1` because higher worker counts regressed on the current measured native path.

## Platform Status

- macOS: native Essentia path exercised locally
- Linux: Rust-only path and build scaffolding are in place; native validation is still open
- Windows: Rust-only path is part of the intended support surface; native Essentia integration is still open

See [docs/platforms.md](docs/platforms.md) for build and release realism per platform.

## Documentation Map

- [docs/README.md](docs/README.md): documentation guide
- [docs/architecture.md](docs/architecture.md): crate boundaries and runtime flow
- [docs/ffi-boundary.md](docs/ffi-boundary.md): native boundary contract
- [docs/feature-schema.md](docs/feature-schema.md): JSONL record shape
- [docs/profiles.md](docs/profiles.md): profile behavior and caveats
- [docs/performance.md](docs/performance.md): current cost model and optimization strategy
- [docs/benchmarking.md](docs/benchmarking.md): benchmark commands and interpretation
- [docs/platforms.md](docs/platforms.md): platform status and native-build expectations
- [docs/licensing.md](docs/licensing.md): AGPL-oriented software licensing and output-data distinction

## Verification

The standard repository quality bar is:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The default GitHub Actions workflow is manual-only via `workflow_dispatch` and currently validates the Rust-only workspace on macOS, Windows, and Linux.

## Licensing

The repository is licensed under `AGPL-3.0-only`.

This is an Essentia-oriented tool and does not attempt to weaken the AGPL implications of using Essentia as the analysis backend. Generated JSONL feature data is documented separately from the software license and is not automatically treated as AGPL solely because this tool produced it.

See:

- [LICENSE](LICENSE)
- [THIRD_PARTY.md](THIRD_PARTY.md)
- [docs/licensing.md](docs/licensing.md)
