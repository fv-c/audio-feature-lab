
# Execution Plan (STRICT)

## Phase 1 — Repository scaffold
- Create workspace
- Define crates
- Add basic CLI skeleton
- Add config loader

## Phase 2 — Domain model
- Implement AnalysisRecord
- Implement FeatureSchema
- Implement Aggregation structure
- Ensure serialization matches json-schema.md

## Phase 3 — Walker
- Recursive scanning
- Filtering extensions
- Cross-platform path handling
- File identity (mtime + size)

## Phase 4 — FFI wrapper
- Minimal C/C++ wrapper
- JSON string return only
- No complex bindings

## Phase 5 — Pipeline
- Streaming processing
- Per-file analysis
- Aggregation
- Immediate JSONL write

## Phase 6 — Profiles
- Implement minimal/default/research
- Validate config constraints

## Phase 7 — CLI
- analyze
- batch
- scan
- validate-config

## Phase 8 — Storage
- JSONL writer
- Append-safe

## Phase 9 — Benchmarks
- Throughput
- Memory
- Profile comparison

## Phase 10 — Documentation
- performance.md
- platforms.md
- licensing.md

## Phase 11 — CI scaffold
- cargo fmt
- clippy
- tests
