# Performance

This repository treats performance as a first-class engineering constraint.

## Current Cost Centers

- the native Essentia call dominates end-to-end analysis time
- `MusicExtractor` currently behaves like a relatively fixed-cost backend across `minimal`, `default`, and `research`
- JSON serialization, walker traversal, and JSONL writing are materially cheaper than native extraction on real runs

## Current Batch Strategy

- batch and scan support bounded worker pools through `[performance].workers`
- worker count is clamped to `available_parallelism()` at runtime to avoid oversubscription
- output order remains stable because records are written on the main thread in input order
- memory stays bounded because jobs are dispatched through a bounded queue and records are flushed progressively

## Measured Optimization Loop

Local measurement on 2026-03-29 against a 4-file WAV corpus on the current macOS Essentia setup showed:

- `workers = 1`: 27.9 s
- `workers = 4`: 31.7 s

That result means higher worker counts are not a safe default on the current native backend path. The built-in profile configs therefore keep `workers = 1`, while the bounded parallel path remains available for explicit benchmarking and platform-specific tuning.

## Practical Guidance

- benchmark worker counts per target machine instead of assuming more threads are faster
- treat native extraction cost as the main optimization target before micro-optimizing Rust-side serialization
- enable frame-level output only when needed, because it increases payload size and backend work
- keep unsupported descriptors omitted rather than approximated, even if that leaves some profiles partially fulfilled

## Next Bottlenecks To Attack

- reduce native per-file work by narrowing the Essentia extraction graph where the upstream API allows it
- benchmark `workers = 2` and other small counts per platform before changing defaults
- add memory measurements for native batch runs alongside throughput numbers
