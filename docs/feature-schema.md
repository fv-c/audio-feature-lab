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

`features`

- family maps for `spectral`, `temporal`, `rhythm`, `tonal`, `dynamics`, `metadata`
- `frame_level`
- `frame_level` is always present and is `null` when disabled

`aggregation`

- nested structure `family -> feature -> statistic`
- vector-valued statistics remain arrays
- current implementation supports `mean` as the serialized statistic

`provenance`

- `backend`
- `boundary`
- `backend_version`

`status`

- backend- or pipeline-provided status details
- usually includes `code`, `success`, `warnings`, and `errors`
- may also include `message` for Rust-side orchestration failures

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
