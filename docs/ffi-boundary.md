# FFI Boundary

The native boundary is intentionally narrow.

## Public Shape

Project-facing Rust facade:

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

The Rust workspace does not expose the Essentia API directly, and the same rule will apply to any future MPEG-7 native implementation.

## Why The Boundary Is JSON-Based

- one call per file keeps cross-language overhead low
- JSON avoids a large shared type graph across Rust and C++
- unsupported descriptors can be omitted naturally without inventing placeholder structs
- Rust remains the owner of the final repository record shape

## Current Payload Contract

The native backend returns one JSON string containing:

- `audio`
- `features`
- `aggregation`
- `status`

Rust then adds:

- `schema`
- `file`
- `analysis`
- `provenance`

This split is deliberate. It keeps the native side focused on analysis, not repository-level bookkeeping.

## Unsafe Code Boundary

- raw declarations live in `crates/afl-essentia-sys`
- `crates/afl-essentia` converts C strings to Rust strings and frees returned memory immediately
- the rest of the workspace only sees safe Rust results

## Current Limitations

- the safe wrapper currently requires UTF-8 file paths because the native API accepts `const char*`
- invalid JSON from the backend is treated as a backend-response failure and converted into an error record
- unsupported descriptors remain omitted with warnings instead of being approximated

## Build Requirements

When the Essentia `native-backend` feature is enabled:

- `ESSENTIA_PREFIX` should point to a local Essentia installation prefix
- `pkg-config` must be able to resolve the Essentia installation
- the wrapper is built through `native/essentia-wrapper/CMakeLists.txt`

The current build script also supports a repository-local fallback at `/tmp/essentia-install` when that path contains a usable Essentia install.

The MPEG-7 path is not yet linked to a native backend. The repository already supports selecting it in config, dispatching to it in Rust, and validating against a conservative declared exact subset (`centroid`, `spread`), but the current result is still an explicit unavailable-backend error rather than a fake implementation.

## Platform Notes

- macOS: current local validation path
- Linux: intended to work through `pkg-config` and the C++ standard library, but not yet validated in repository automation
- Windows: Rust workspace support exists, but native Essentia linking and runtime packaging still require explicit platform-specific work
