# Benchmarking

The repository includes a Phase 9 benchmark suite focused on the parts of the system that are already real and measurable.

Run the current suite with:

```sh
cargo bench -p audio-feature-lab-core --bench phase9
```

Local native analysis is now available separately with:

```sh
cargo run -p audio-feature-lab --features native-backend -- analyze fixtures/audio/short-stereo-44k.wav
```

## What is measured now

- walker throughput on flat and nested corpora
- per-file Rust pipeline overhead with one backend call per file
- batch streaming throughput without accumulating the dataset in memory
- JSONL serialization and write throughput
- baseline skip-policy cost using `mtime + size`
- relative Rust-side overhead for `minimal`, `default`, and `research`

## What is not claimed yet

- true Essentia extraction cost
- end-to-end descriptor throughput with the native backend enabled
- real profile compute cost for heavy descriptors such as `spectral_peaks`
- resident memory measurements from a native backend run

Those measurements still need dedicated benchmark targets even though local native analysis is now available. They must be added as explicit end-to-end benchmarks rather than inferred from the current Rust-side suite.

## Fixture Strategy

The benchmark harness uses small non-silent repository fixtures under `fixtures/audio/` and expands them into temporary corpora during benchmark setup. This keeps the scenarios reproducible without storing a large benchmark corpus in git.

For walker and Rust-side pipeline benchmarks, some generated corpus entries reuse the small WAV seed bytes under different audio extensions. This is intentional and honest: those scenarios measure scan/filtering and orchestration overhead, not real codec decoding or Essentia extraction.

## Reading The Results

- `walker` isolates filesystem traversal and filtering cost
- `pipeline_single_file` measures Rust orchestration plus payload parsing and record construction
- `pipeline_batch` measures streaming scan-to-record throughput with a null sink
- `jsonl` measures deterministic JSONL serialization and line writing cost
- `skip_logic` measures the baseline unchanged-file detection policy the future cache layer will use

## Optimization Loop

Use the benchmark suite as part of the required loop:

1. establish a baseline with the current branch
2. change one bottleneck candidate at a time
3. re-run the relevant benchmark group
4. compare results before making broader claims

This suite is intentionally narrow and honest: it measures the Rust-side system as it exists today, not the final native analysis cost.
