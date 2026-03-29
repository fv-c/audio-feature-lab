# Benchmarking

The repository includes a benchmark suite that separates always-available Rust-side measurements from feature-gated native Essentia measurements.

Run the Rust-side suite with:

```sh
cargo bench -p audio-feature-lab-core --bench phase9
```

Run the native end-to-end groups with:

```sh
cargo bench -p audio-feature-lab-core --bench phase9 --features native-backend
```

If you only want the native groups, filter the benchmark binary to keep Criterion runs bounded:

```sh
cargo bench -p audio-feature-lab-core --bench phase9 --features native-backend pipeline_single_file_native
```

Local native analysis is also available directly with:

```sh
cargo run -p audio-feature-lab --features native-backend -- analyze fixtures/audio/short-stereo-44k.wav
```

## Preconditions

- Rust-only benchmark groups require only the normal Rust workspace
- native groups require `--features native-backend` and a local Essentia installation visible through `ESSENTIA_PREFIX` or the `/tmp/essentia-install` fallback used by this repository
- benchmark numbers are only comparable when the same backend availability, build profile, and local machine conditions are preserved
- for operationally accurate native runs today, keep `aggregation.statistics = ["mean"]`

## What is measured now

- walker throughput on flat and nested corpora
- per-file Rust pipeline overhead with one backend call per file
- batch streaming throughput without accumulating the dataset in memory
- JSONL serialization and write throughput
- baseline skip-policy cost using `mtime + size`
- relative Rust-side overhead for `minimal`, `default`, and `research`
- end-to-end native single-file cost for `minimal`, `default`, and `research` when `native-backend` is enabled
- end-to-end native batch scan cost with streaming sink writes when `native-backend` is enabled
- bounded-worker batch comparisons on both the Rust-only and native benchmark paths

## What is not claimed yet

- Windows and Linux native benchmark baselines
- real profile compute cost for unsupported heavy descriptors such as `spectral_peaks`
- resident memory measurements from a native backend run

The native benchmarks are real, but they only measure descriptors that the current `MusicExtractor`-based backend actually returns. Unsupported descriptors remain omitted with warnings and must not be inferred from these numbers.

## Fixtures

The benchmark harness uses small non-silent repository fixtures under `fixtures/audio/` and expands them into temporary corpora during benchmark setup. This keeps the scenarios reproducible without storing a large benchmark corpus in git.

For walker and Rust-side pipeline benchmarks, some generated corpus entries reuse the small WAV seed bytes under different audio extensions. This is intentional and honest: those scenarios measure scan/filtering and orchestration overhead, not real codec decoding or Essentia extraction.

For native end-to-end benchmarks, the generated corpus uses real `.wav` copies only. That avoids pretending codec coverage the fixture set does not provide yet.

## Reading The Results

- `walker` isolates filesystem traversal and filtering cost
- `pipeline_single_file` measures Rust orchestration plus payload parsing and record construction
- `pipeline_batch` measures streaming scan-to-record throughput with a null sink
- `jsonl` measures deterministic JSONL serialization and line writing cost
- `skip_logic` measures the baseline unchanged-file detection policy the future cache layer will use
- `pipeline_single_file_native` measures one real Essentia-backed analysis call plus Rust-side record construction
- `pipeline_batch_native` measures real scan-to-analysis streaming throughput against the native backend
- `pipeline_batch_workers` and `pipeline_batch_native_workers` compare bounded worker counts instead of assuming parallelism is always a win

## Optimization Loop

Use the benchmark suite as part of the repository optimization loop:

1. establish a baseline with the current branch
2. change one bottleneck candidate at a time
3. re-run the relevant benchmark group
4. compare results before making broader claims

This suite is intentionally explicit about scope: the Rust-only groups stay available everywhere, while the native groups measure the actual local Essentia integration path without claiming unsupported descriptors or cross-platform parity that has not been validated yet.

The current local optimization loop also showed that higher worker counts can regress throughput on the native Essentia path. See `docs/performance.md` before changing built-in worker defaults.
