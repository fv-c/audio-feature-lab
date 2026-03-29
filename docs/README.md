# Documentation

This directory documents the current repository as an operational tool.

Use the top-level [README.md](../README.md) first for a quick start, then use the documents here for the specific area you need:

- [architecture.md](architecture.md): workspace layout, runtime flow, and current boundaries between Rust and Essentia
- [feature-schema.md](feature-schema.md): exact JSONL record shape and current schema caveats
- [profiles.md](profiles.md): shipped profile configs and what they request today
- [performance.md](performance.md): current cost model, worker guidance, and optimization priorities
- [benchmarking.md](benchmarking.md): benchmark commands, scope, and interpretation
- [platforms.md](platforms.md): platform support realism and native-build expectations
- [ffi-boundary.md](ffi-boundary.md): native ABI contract and build requirements
- [licensing.md](licensing.md): AGPL-oriented software licensing and output-data distinction

`docs/agent/` remains the authoritative design specification for repository constraints. The documents in `docs/` describe the implementation that exists today. When there is a gap between the ideal target and the current implementation, these documents should narrow the public surface to the operational subset and list the deferred pieces explicitly instead of implying work that has not been completed.
