# audio-feature-lab

Production-grade, performance-oriented Rust workspace for large-scale audio feature extraction with Essentia as backend.

Current state: workspace scaffold, validated config system, domain/JSON output model, filesystem walker with file identity baseline, a real feature-gated Essentia native boundary, a streaming core pipeline, append-friendly JSONL storage, a sober CLI surface, and a Criterion-based benchmark suite covering both Rust-side costs and feature-gated native end-to-end paths.

## Workspace

- `crates/audio-feature-lab-cli`: command-line entry point with analyze, batch, scan, backend-info, schema, and config commands
- `crates/audio-feature-lab-config`: minimal config loader for named profiles
- `crates/audio-feature-lab-core`: Rust-side domain model, JSON output model, walker/file identity layer, streaming pipeline, and JSONL writer
- `crates/audio-feature-lab-core/benches/phase9.rs`: benchmark target for walker, pipeline, JSONL, skip-policy, and profile-overhead measurements
- `crates/afl-essentia-sys`: raw C FFI declarations plus build script for the native wrapper
- `crates/afl-essentia`: safe Rust wrapper around the native boundary
- `crates/audio-feature-lab-ffi`: project-facing façade for the preferred backend API
- `native/essentia-wrapper`: C++ wrapper and CMake project for local Essentia integration
- `configs/*.toml`: minimal, default, and research example profile files
- `fixtures/audio/*.wav`: lightweight benchmark and test seed inputs
- `docs/benchmarking.md`: benchmark scope, commands, and interpretation notes
- `docs/agent`: authoritative project constraints and execution plan

## Scope

The repository now includes the workspace scaffold, typed profile configuration, a deterministic JSON record model for analysis output, a recursive filesystem walker with metadata-based file identity, a narrow native Essentia backend that keeps the FFI at a single JSON-string-per-file contract, a streaming pipeline that processes files one by one into JSONL-ready records, an append-friendly JSONL storage layer with line-by-line validation helpers, a sober CLI that exposes single-file analysis, batch analysis, dry-run scanning, backend inspection, schema inspection, and config validation, and a benchmark suite that measures the Rust-side walker, pipeline, JSONL, skip-policy, and profile-overhead behavior.

## Native backend

The real Essentia backend is feature-gated so the standard Rust workspace can still build and test without a local native toolchain:

```sh
cargo run -p audio-feature-lab --features native-backend -- backend-info
cargo run -p audio-feature-lab --features native-backend -- analyze fixtures/audio/short-stereo-44k.wav
```

Local build assumptions validated in this repository today:

- Essentia installed under `ESSENTIA_PREFIX`
- if `ESSENTIA_PREFIX` is not set, the build script falls back to `/tmp/essentia-install` when that local install exists
- the native wrapper is built through `native/essentia-wrapper/CMakeLists.txt`

Current native backend behavior:

- one Essentia call per file via `MusicExtractor`
- deterministic JSON payload returned across the FFI boundary
- explicit warnings for requested features that the current backend does not emit yet
- `frame_level = true` is supported for the subset of descriptors that `MusicExtractor` exposes as frame sequences; unsupported frame-level descriptors remain omitted with warnings

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

Current explicit limitations:

- some requested features are still omitted with warnings rather than silently approximated, notably `flatness`, `onset_strength`, `contrast`, `inharmonicity`, and `spectral_peaks`
- the native path has been validated locally on macOS; Windows and Linux still need their own native dependency passes
- native Criterion groups are now available behind `--features native-backend`, but they are environment-dependent and substantially slower than the Rust-only groups
