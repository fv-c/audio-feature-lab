
# Architecture

## Separation of concerns

Rust layer:
- orchestration
- filesystem IO
- configuration
- caching
- serialization
- CLI
- batching

Essentia layer:
- descriptor extraction
- signal processing

## Data flow

scan -> filter -> analyze (FFI) -> aggregate -> serialize (JSONL)

## Key design invariants

- No global state accumulation
- Each file processed independently
- Deterministic output
- Stable schema

## FFI philosophy

- Single call per file preferred
- JSON boundary instead of structured FFI types
- Avoid cross-language complexity
