# Feature Schema

Primary storage format is JSONL. Each line is one analysis record for one audio file.

## Top-Level Shape

Records are serialized with deterministic top-level ordering:

1. `schema`
2. `file`
3. `audio`
4. `analysis`
5. `features`
6. `aggregation`
7. `provenance`
8. `status`

The top-level blocks always exist, even when some inner blocks are sparse.

Representative shape:

```json
{
  "schema": {},
  "file": {
    "path": "/abs/or/input/path.wav",
    "relative_path": "path.wav",
    "identity": {
      "modified_unix_nanos": 0,
      "size_bytes": 0
    }
  },
  "audio": {},
  "analysis": {
    "backend": "essentia",
    "profile": "default",
    "frame_level": false,
    "requested_families": ["spectral"],
    "requested_features": ["centroid"],
    "aggregation_statistics": ["mean"]
  },
  "features": {
    "spectral": {},
    "temporal": {},
    "rhythm": {},
    "tonal": {},
    "dynamics": {},
    "metadata": {},
    "frame_level": null
  },
  "aggregation": {
    "spectral": {
      "centroid": {
        "mean": 123.0
      }
    },
    "temporal": {},
    "rhythm": {},
    "tonal": {},
    "dynamics": {},
    "metadata": {}
  },
  "provenance": {
    "backend": "essentia",
    "boundary": "json_string",
    "backend_version": "..."
  },
  "status": {
    "code": "ok",
    "success": true,
    "warnings": [],
    "errors": []
  }
}
```

## Current Implemented Blocks

`schema`

- currently present as a reserved block
- deterministic position is already enforced
- stable inner fields are still future work

`file`

- `path`
- `relative_path`
- `identity.modified_unix_nanos`
- `identity.size_bytes`
- canonical path and content hash are not implemented yet

`audio`

- backend-provided audio metadata when available
- typically includes fields such as `sample_rate`, `channels`, and `duration_seconds`

`analysis`

- `backend`
- `profile`
- `frame_level`
- `requested_families`
- `requested_features`
- `aggregation_statistics`
- analysis timestamp is not implemented yet

`features`

- family maps for `spectral`, `temporal`, `rhythm`, `tonal`, `dynamics`, `metadata`
- these maps contain the available file-level descriptor values for the current file
- `frame_level`
- `frame_level` is always present and is `null` when disabled

`aggregation`

- nested structure `family -> feature -> statistic`
- vector-valued statistics remain arrays
- the record model supports the full allowed statistic set
- the current native Essentia backend emits `mean` only
- with the current backend, `aggregation.*.*.mean` often mirrors the corresponding `features.<family>.<feature>` value

`provenance`

- `backend`
- `boundary`
- `backend_version`

`status`

- backend- or pipeline-provided status details
- usually includes `code`, `success`, `warnings`, and `errors`
- may also include `message` for Rust-side orchestration failures
- failed analyses still produce records, which is useful for corpus-scale runs

## Aggregation Naming

Aggregation keys remain hierarchical:

- `aggregation.spectral.centroid.mean`
- `aggregation.spectral.mfcc.mean`

Flattened keys such as `mfcc_01_mean` are not produced by the current implementation.

## Feature Value Rules

- scalar feature values serialize as numbers
- vector feature values serialize as arrays
- frame-level scalar sequences serialize as arrays of numbers
- frame-level vector sequences serialize as arrays of arrays

Unsupported descriptors are omitted. They are not renamed, approximated, or flattened.

## Current Coverage Caveat

The controlled vocabulary is broader than the set of descriptors the current `MusicExtractor` wrapper emits. Requested but unsupported descriptors remain absent from `features` and `aggregation`, and the backend reports warnings instead of silently fabricating values.

## Current Implementation Gaps Against The Target Spec

- `schema` is reserved but still sparse
- `file` does not yet include canonical path or optional fingerprint fields
- `analysis` does not yet include an analysis timestamp
- full aggregation vocabulary is modeled, but the native backend still operates on `mean` only
