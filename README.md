# audio-feature-lab

`audio-feature-lab` is a Rust command-line tool for corpus-scale audio feature extraction with Essentia as the analysis backend.

It is built for repeatable batch work:

- recursive folder scanning
- typed TOML configuration
- one-record-per-file JSONL output
- bounded-memory batch processing
- deterministic nested serialization
- explicit per-file provenance and status

The repository is maintained as a software tool. Public commands, shipped configs, and documentation are meant to reflect the operational implementation, not an aspirational design target.

## Current Status

Today the only supported public backend is Essentia.

What is operational:

- CLI commands for single-file analysis, batch analysis, dry-run scanning, config validation, backend inspection, and schema explanation
- shipped `minimal`, `default`, and `research` profiles constrained to the descriptors the current Essentia wrapper maps exactly
- streaming JSONL output with one record per analyzed file
- bounded worker-pool batch orchestration
- file identity extraction based on `mtime + size`
- local benchmark suite for walker, pipeline, JSONL, and native runs

What is intentionally deferred:

- canonical path and content hash fields in the `file` block
- persistent cross-run cache storage
- aggregation statistics beyond `mean` on the native backend
- native validation on Linux and Windows
- broader design-target descriptors that the current Essentia wrapper does not yet expose

The current public tool surface does not advertise or ship configs for unsupported descriptors.

## Build And Install

Rust-only commands build with a normal Rust toolchain:

```sh
cargo build -p audio-feature-lab --release
```

Real analysis also requires a local Essentia installation reachable by the native build script:

- set `ESSENTIA_PREFIX=/path/to/essentia/prefix`, or
- provide the repository-local fallback at `/tmp/essentia-install`

Then build the native-enabled binary:

```sh
cargo build -p audio-feature-lab --release --features native-backend
```

After that, use the built binary directly:

```sh
./target/release/audio-feature-lab --help
```

If you prefer a local installed command instead of a path under `target/`, install the CLI crate into Cargo's binary directory:

```sh
cargo install --path crates/audio-feature-lab-cli --features native-backend
```

## Quick Start

Commands that do not require Essentia:

```sh
audio-feature-lab validate-config configs/default.toml
audio-feature-lab scan fixtures/audio --dry-run
audio-feature-lab schema
audio-feature-lab backend-info
```

Real analysis with a local Essentia installation:

```sh
audio-feature-lab analyze fixtures/audio/short-stereo-44k.wav --config configs/default.toml
audio-feature-lab batch /path/to/corpus --config configs/default.toml --output results.jsonl
```

Batch progress and summaries are written to `stderr`. JSONL records go to `stdout` or to the file passed with `--output`.

## CLI Surface

Supported commands:

- `analyze <file>`
- `batch <path>`
- `scan <path> --dry-run`
- `backend-info`
- `schema`
- `validate-config <file>`
- `profiles`
- `explain-schema`

Operational notes:

- `analyze` writes one JSON record
- `batch` accepts either a single file or a directory tree
- `scan --dry-run` only lists candidate files; it does not invoke Essentia
- batch progress lives on `stderr`, so redirecting `stdout` still produces clean JSONL
- analysis failures still produce per-file records with `status.success = false`
- `backend-info` reports the currently supported descriptor surface, frame-level subset, and supported aggregation statistics

## Configuration

Configuration is TOML and is validated before analysis starts.

The repository ships three built-in profiles:

- `configs/minimal.toml`
- `configs/default.toml`
- `configs/research.toml`

Current behavior:

- all shipped configs use `[backend].name = "essentia"`
- shipped configs only request descriptors mapped by the current Essentia wrapper
- frame-level extraction is disabled by default
- shipped configs currently use `aggregation.statistics = ["mean"]`
- config validation rejects feature names and aggregation statistics that are not operational on the current public backend

Use `audio-feature-lab profiles` for a quick profile summary, or see [docs/profiles.md](docs/profiles.md) for the exact shipped scope.

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
- `features.<family>` carries file-level descriptor values for the current file
- `features.frame_level` is always present
- when frame-level extraction is disabled, `features.frame_level` is `null`
- unsupported or unavailable values are omitted rather than fabricated
- failed analyses are still represented as records through the `status` block

See [docs/feature-schema.md](docs/feature-schema.md) for the current record shape.

## Backend Surface

When the binary is built with `native-backend`:

- Rust handles orchestration, configuration, walker logic, schema ownership, JSONL writing, and user-facing errors
- Essentia handles descriptor extraction
- the FFI boundary remains one JSON-returning call per file
- the current native wrapper accepts only `mean` aggregation
- frame-level output is available only for the descriptors the wrapper can actually emit frame-by-frame

Broader design-target descriptors remain deferred until the backend can support them credibly. They are not part of the shipped operational surface today.

## Platform Status

- macOS: native Essentia path exercised locally
- Linux: Rust-only path and native build scaffolding exist; native validation is still open
- Windows: Rust-only path is part of the intended support surface; native Essentia integration is still open

See [docs/platforms.md](docs/platforms.md) for platform-specific build realism.

## Documentation Map

- [docs/README.md](docs/README.md): documentation guide
- [docs/architecture.md](docs/architecture.md): crate boundaries and runtime flow
- [docs/ffi-boundary.md](docs/ffi-boundary.md): native boundary contract
- [docs/feature-schema.md](docs/feature-schema.md): JSONL record shape
- [docs/profiles.md](docs/profiles.md): shipped profile behavior
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
