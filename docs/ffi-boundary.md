# FFI Boundary

The native boundary is intentionally narrow.

## Public Shape

Project-facing Rust facade:

- `known_backends()`
- `backend_status(backend)`
- `backend_version(backend)`
- `analyze_file(backend, path, config_json)`

Per-backend native wrapper shape:

- `backend_version()`
- `analyze_file(path, config_json)`
- `free_string()`

Native C ABI:

- `afl_essentia_backend_version()`
- `afl_essentia_analyze_file(const char* path, const char* config_json)`
- `afl_essentia_free_string(char* value)`

The Rust workspace does not expose the Essentia API directly.

## Why The Boundary Is JSON-Based

- one call per file keeps cross-language overhead low
- JSON avoids a large shared type graph across Rust and C++
- Rust keeps ownership of the repository record shape
- backend-local omissions can be represented without proliferating unsafe bindings

## Current Payload Contract

The native backend returns one JSON string containing:

- `audio`
- `features`
- `aggregation`
- `status`

Rust adds:

- `schema`
- `file`
- `analysis`
- `provenance`

## Current Essentia Behavior

The current C++ wrapper is conservative and measurable:

- one native call per file
- `mean` aggregation only
- file-level descriptor values in `features.<family>`
- optional frame-level data only for descriptors the backend can actually emit frame-by-frame
- structured backend-side failures

The important product rule is now upstream of the boundary:

- configs that request unsupported descriptors are rejected before the backend call
- configs that request unsupported aggregation statistics are rejected before the backend call

Runtime warnings remain useful for unexpected omissions or file-specific failures, but unsupported public states should no longer reach the native layer through shipped configs or validated operator configs.

## Unsafe Code Boundary

- raw declarations live in `crates/afl-essentia-sys`
- `crates/afl-essentia` converts C strings to Rust strings and frees returned memory immediately
- the rest of the workspace only sees safe Rust results

## Current Limitations

- the safe wrapper currently requires UTF-8 file paths because the native API accepts `const char*`
- the current public Essentia backend exposes only `mean` in `aggregation`
- frame-level output is a supported subset, not a full descriptor-wide guarantee
- the wrapper is validated locally on macOS only

## Build Requirements

When the Essentia `native-backend` feature is enabled:

- `ESSENTIA_PREFIX` should point to a local Essentia installation prefix, or
- `/tmp/essentia-install` can be used as the repository-local fallback when present
- `pkg-config` must resolve the Essentia installation
- the wrapper is built through `native/essentia-wrapper/CMakeLists.txt`

## Future Boundary Rules

Any future backend must follow the same rules:

- narrow per-file API
- JSON payload boundary
- explicit capability gating
- no public exposure until descriptor coverage is credible against the controlled vocabulary
