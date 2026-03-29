# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: validated Rust-side workspace, real feature-gated Essentia integration, backend-aware pipeline/configuration, streaming JSONL pipeline, sober CLI, Criterion benchmarks, and cross-platform Rust CI. The native Essentia backend is working locally, while the MPEG-7 path is now a real selectable scaffold in config and provenance but does not yet have a linked native implementation.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point with analyze, batch, scan, backend-info, schema, and config commands
- `crates/audio-feature-lab-config`: typed TOML config loader, backend selection, and profile validation
- `crates/audio-feature-lab-core`: Rust-side domain model, JSON output model, walker/file identity layer, streaming pipeline, and JSONL writer
- `crates/audio-feature-lab-core/benches/phase9.rs`: benchmark target for walker, pipeline, JSONL, skip-policy, and profile-overhead measurements
- `crates/afl-essentia-sys`: raw C FFI declarations plus build script for the native wrapper
- `crates/afl-essentia`: safe Rust wrapper around the native boundary
- `crates/audio-feature-lab-ffi`: project-facing façade for the preferred backend API
- `native/essentia-wrapper`: C++ wrapper and CMake project for local Essentia integration
- `configs/*.toml`: minimal, default, and research example profile files
- `fixtures/audio/*.wav`: lightweight benchmark and test seed inputs
- `docs/architecture.md`: implementation-oriented overview of crate boundaries and data flow
- `docs/benchmarking.md`: benchmark scope, commands, and interpretation notes
- `docs/feature-schema.md`: current JSONL schema and record-shape notes
- `docs/ffi-boundary.md`: native boundary contract and local build requirements
- `docs/licensing.md`: engineering-oriented AGPL and output-data notes
- `docs/performance.md`: current bottlenecks, concurrency strategy, and measured optimization-loop results
- `docs/platforms.md`: platform-specific support and native-build expectations
- `docs/profiles.md`: built-in profiles and current backend coverage caveats
- `docs/agent`: authoritative project constraints and execution plan

## Quick Start

Rust-only commands that do not require Essentia:

```sh
cargo run -p audio-feature-lab -- validate-config configs/default.toml
cargo run -p audio-feature-lab -- scan fixtures/audio --dry-run
cargo run -p audio-feature-lab -- schema
cargo run -p audio-feature-lab -- backend-info
```

Native analysis with a local Essentia installation:

```sh
export ESSENTIA_PREFIX=/path/to/essentia/prefix
cargo run -p audio-feature-lab --features native-backend -- backend-info
cargo run -p audio-feature-lab --features native-backend -- analyze fixtures/audio/short-stereo-44k.wav
cargo run -p audio-feature-lab --features native-backend -- batch fixtures/audio --output results.jsonl
```

## Native backend

The Essentia backend is feature-gated so the standard workspace can build and test without a local native toolchain. When `native-backend` is enabled, Rust still performs orchestration, schema construction, streaming output, and error handling; Essentia stays behind one JSON-returning call per file.

The repository is now backend-aware:

- shipped configs use `[backend].name = "essentia"`
- `analysis.backend` and `provenance.backend` record the selected backend
- `backend-info` reports both the working Essentia path and the current MPEG-7 scaffold state
- selecting `mpeg7` is already supported at config and pipeline level
- `mpeg7` currently declares only a conservative exact feature subset: `centroid`, `spread`
- analysis with `mpeg7` still fails honestly because no native MPEG-7 implementation is linked yet

Current local build assumptions:

- Essentia installed under `ESSENTIA_PREFIX`
- if `ESSENTIA_PREFIX` is not set, the build script falls back to `/tmp/essentia-install` when that local install exists
- the native wrapper is built through `native/essentia-wrapper/CMakeLists.txt`
- the default GitHub Actions workflow does not enable `native-backend`, because generic CI runners do not provide a consistent Essentia installation

Current native backend behavior:

- one Essentia call per file via `MusicExtractor`
- deterministic JSON payload returned across the FFI boundary
- explicit warnings for requested features that the current backend does not emit yet
- `frame_level = true` is supported for the subset of descriptors that `MusicExtractor` exposes as frame sequences; unsupported frame-level descriptors remain omitted with warnings
- batch worker count is configurable through `[performance].workers`, but built-in profile configs keep `workers = 1` because the current local Essentia setup regressed at higher counts during measurement

Current supported aggregated features include:

- spectral: `centroid`, `spread`, `skewness`, `kurtosis`, `rolloff`, `flux`, `energy`, `entropy`, `complexity`, `hfc`, `strong_peak`, `dissonance`, `mfcc`, `bark_bands`, `mel_bands`, `erb_bands`, `gfcc`
- temporal: `zcr`, `rms`, `dynamic_range`
- rhythm: `onset_rate`, `tempo`, `beat_period`
- tonal: `hpcp`, `chroma`, `key_strength`, `tuning_frequency`
- dynamics: `loudness`, `loudness_ebu`, `dynamic_complexity`
- metadata: `duration`, `silence_ratio`, `active_ratio`

Current supported frame-level features include:

- spectral: `centroid`, `spread`, `skewness`, `kurtosis`, `rolloff`, `flux`, `energy`, `entropy`, `complexity`, `hfc`, `strong_peak`, `dissonance`, `mfcc`, `bark_bands`, `mel_bands`, `erb_bands`, `gfcc`
- temporal: `zcr`, `rms`
- tonal: `hpcp`, `chroma`
- dynamics: `loudness_ebu`

## CLI Surface

- `analyze <file>`: analyze one file and emit one JSON record
- `batch <path>`: analyze one file or a walked directory and emit JSONL
- `scan <path> --dry-run`: show files that would be analyzed
- `backend-info`: report backend availability and version details
- `schema`: print the current record skeleton and frame-level note
- `validate-config <file>`: validate a TOML config with profile and worker details
- `profiles`: list built-in profiles
- `explain-schema`: print a concise explanation of the JSONL shape

Batch progress and summaries are written to `stderr`. JSONL always stays on `stdout` or the `--output` file.

## Output Model

- output format is JSONL, one record per analyzed file
- top-level blocks are always emitted in deterministic order:
  - `schema`
  - `file`
  - `audio`
  - `analysis`
  - `features`
  - `aggregation`
  - `provenance`
  - `status`
- aggregation remains hierarchical as `aggregation.<family>.<feature>.<statistic>`
- vector-valued features and statistics remain arrays
- `features.frame_level` is always present and is `null` when frame-level extraction is disabled

See [docs/feature-schema.md](docs/feature-schema.md) for the current implemented record details.

## Profiles And Performance

The repository ships three built-in configs:

- `configs/minimal.toml`
- `configs/default.toml`
- `configs/research.toml`

All three currently use `[backend].name = "essentia"` and keep `[performance].workers = 1`. That is deliberate: on the current measured local macOS native setup, a 4-worker batch run regressed relative to a 1-worker run. The bounded parallel path remains available for explicit measurement and tuning.

Current measured result documented in [docs/performance.md](docs/performance.md):

- `workers = 1`: `27.9 s`
- `workers = 4`: `31.7 s`

See also:

- [docs/profiles.md](docs/profiles.md)
- [docs/benchmarking.md](docs/benchmarking.md)
- [docs/performance.md](docs/performance.md)

The GitHub Actions workflow is currently manual-only via `workflow_dispatch`, so CI runs are explicit rather than automatic on every push or pull request.

## Licensing

The workspace is licensed under `AGPL-3.0-only`. This is an Essentia-oriented project and does not attempt to isolate or weaken the AGPL implications of using Essentia as the analysis backend.

Generated JSONL feature data is documented separately from the software license. It is not treated as AGPL by default, provided it does not embed software source code or implementation. Rights on the original audio material still apply.

See:

- [LICENSE](LICENSE)
- [THIRD_PARTY.md](THIRD_PARTY.md)
- [docs/licensing.md](docs/licensing.md)

## Current Limitations

- some requested features are still omitted with warnings rather than silently approximated, notably `flatness`, `onset_strength`, `contrast`, `inharmonicity`, and `spectral_peaks`
- aggregated statistics are currently implemented as `mean` only
- MPEG-7 is selectable in config, exposed in CLI/backend info, and has a declared exact subset for validation, but no native MPEG-7 backend is linked yet
- the `schema` block is present and reserved, but not yet populated with stable inner fields
- the `file` block currently stores `path`, `relative_path`, and the baseline identity `{ modified_unix_nanos, size_bytes }`; canonical paths and content hashes are still future work
- the native path has been validated locally on macOS; Linux and Windows still need their own native dependency passes
- native Criterion groups are now available behind `--features native-backend`, but they are environment-dependent and substantially slower than the Rust-only groups
- the current native wrapper expects UTF-8 file paths at the Rust/C boundary

See [docs/platforms.md](docs/platforms.md) for the platform-specific TODO list.
