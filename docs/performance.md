# Performance

This repository treats performance as a first-class engineering constraint.

## Design Priorities

- stream records directly to JSONL sinks instead of accumulating the corpus in memory
- serialize backend config once per pipeline instance
- cache `backend_version()` once per pipeline instance
- prefer one native call per file over chatty per-descriptor or per-frame FFI
- keep batch concurrency bounded and clamp it to `available_parallelism()`
- preserve output order while writing records progressively on the main thread
- keep expensive payloads opt-in, especially frame-level output

## What Is Implemented Today

- recursive walker with bounded metadata work
- one-record-per-file JSONL writing
- bounded worker pool for batch mode
- one backend call per file
- deterministic record serialization
- benchmark coverage for walker, JSONL, pipeline overhead, skip-logic groundwork, and native runs

## Current Cost Centers

- the native Essentia call dominates end-to-end analysis time
- `MusicExtractor` behaves more like a relatively fixed-cost extractor than a tightly profile-scaled one on the current validated path
- JSON serialization, walker traversal, and JSONL writing are materially cheaper than native extraction on real runs

## Current Batch Strategy

- batch and scan support bounded worker pools through `[performance].workers`
- worker count is clamped to `available_parallelism()` at runtime
- output order remains stable because records are written on the main thread in input order
- memory stays bounded because jobs are dispatched through a bounded queue and records are flushed progressively

## Measured Optimization Loop

Local measurement on 2026-03-29 against a 4-file WAV corpus on the current macOS Essentia setup showed:

- `workers = 1`: 27.9 s
- `workers = 4`: 31.7 s

That result means higher worker counts are not a safe default on the current native path. The shipped configs therefore keep `workers = 1`.

## Product-Surface Implications

Performance measurements are only meaningful if the product surface matches the backend’s real capabilities.

That now means:

- shipped profiles request only descriptors the backend maps today
- unsupported descriptors are rejected during config validation instead of inflating runs with warning-heavy partial outputs
- unsupported aggregation statistics are rejected during config validation instead of failing later inside the native layer

This change matters for measurement quality, not just user experience. A benchmark on a profile full of impossible descriptors is not an honest benchmark.

## What Is Not Finished Yet

- persistent cross-run cache storage
- Linux and Windows native benchmark baselines
- resident-memory measurements for long native runs
- native extraction narrowing beyond the current `MusicExtractor` path

## Practical Guidance

- benchmark worker counts per target machine instead of assuming more threads are faster
- treat native extraction cost as the main optimization target before micro-optimizing Rust-side serialization
- enable frame-level output only when needed, because it increases payload size and backend work
- keep deferred descriptors out of operational runs until they are actually implemented
