# Benchmark Suite

The executable benchmark targets live under the workspace crates so they can use the real crate APIs directly.

Current Phase 9 benchmark target:

- `crates/audio-feature-lab-core/benches/phase9.rs`

Run it with:

```sh
cargo bench -p audio-feature-lab-core --bench phase9
```

The suite currently measures:

- filesystem walker throughput
- Rust-side single-file pipeline overhead
- Rust-side batch pipeline throughput
- JSONL serialization and write throughput
- baseline mtime+size skip-logic lookup cost
- relative Rust-side overhead across `minimal`, `default`, and `research`

Important limitation:

- these benchmarks do not claim to measure real Essentia descriptor extraction yet
- backend-dependent benchmarks are currently exercised through a stable fake backend that returns deterministic JSON payloads
- this keeps the suite honest and reproducible until local Essentia integration is implemented
