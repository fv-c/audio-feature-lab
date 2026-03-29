# Fixtures

This directory contains lightweight repository fixtures used by tests and benchmarks.

Current contents:

- `audio/short-mono-8k.wav`
- `audio/short-stereo-44k.wav`

These files are intentionally small so they are safe to keep in git and cheap to copy into temporary benchmark corpora.

They are also intentionally non-silent, because Essentia may abort on completely silent inputs and that would make native smoke tests and future real-analysis benchmarks misleading.

Benchmark note:

- the Phase 9 benchmark harness may copy these WAV seed files under other audio extensions when it needs extension-filter and orchestration scenarios
- that does not claim codec correctness for those renamed files
- real backend-dependent fixture expansion will need genuine per-format samples later

Current limitation:

- compressed-format fixtures are not yet included
- the benchmark suite currently focuses on walker, storage, and Rust-side pipeline overhead
- real backend-dependent corpus fixtures still need to be expanded across formats and platforms before end-to-end benchmark claims are made
