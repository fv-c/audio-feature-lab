
# JSON Schema (STRICT)

Each line = one JSON object.

## Top-level structure

{
  "schema": {...},
  "file": {...},
  "audio": {...},
  "analysis": {...},
  "features": {...},
  "aggregation": {...},
  "provenance": {...},
  "status": {...}
}

## Aggregation structure

aggregation.<family>.<feature>.<statistic>

## Rules

- deterministic key ordering
- no missing mandatory blocks
- no flattening of hierarchical keys
- vector features remain arrays

## Example

aggregation.spectral.centroid.mean
aggregation.spectral.mfcc.mean = [ ... ]
