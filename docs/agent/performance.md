
# Performance (CRITICAL)

## Core principles

- streaming processing
- no full dataset in memory
- minimal allocations
- bounded concurrency
- avoid repeated work

## Key constraints

- JSONL written incrementally
- one analysis call per file preferred
- reuse config and buffers

## Optimization loop

1. baseline
2. benchmark
3. profile
4. optimize
5. re-measure

## Anti-patterns

- loading entire dataset in RAM
- repeated FFI calls per frame
- dynamic schema generation
