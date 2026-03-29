
# AGENTS

## Mission
Build a production-grade, performance-oriented Rust system for large-scale audio feature extraction using Essentia as backend.

## Non-negotiable constraints
- License: AGPL-3.0 (see docs/agent/licensing.md)
- Cross-platform: macOS (universal), Windows, Linux (see docs/agent/platforms.md)
- Output: JSONL (strict schema, see docs/agent/json-schema.md)
- Feature naming: strictly controlled vocabulary (see docs/agent/features-vocabulary.md)
- Profiles: minimal / default / research (see docs/agent/profiles.md)
- Performance-first engineering (see docs/agent/performance.md)
- Minimal FFI boundary (see docs/agent/ffi-boundary.md)

## Architecture entry point
See docs/agent/architecture.md

## Execution plan (MANDATORY ORDER)
Follow docs/agent/execution-plan.md

## Working rules
- Never invent new feature names
- Never flatten vector features unless explicitly requested
- Never accumulate full dataset in memory
- Prefer streaming design everywhere
- Keep Rust/native boundary minimal and measurable
- Before changing code, read AGENTS.md and the referenced docs. Then implement only the requested phase. 
- Treat `audio-feature-lab` as a finished tool, not as an internal prototype.
- Documentation updates are part of the definition of done.
- Any user-visible change must update documentation organically:
  - always update `README.md`
  - update every affected file under `docs/` in the same turn when behavior, commands, schema, platform support, performance guidance, or limitations change
  - update usage examples, caveats, and cross-links when commands, config, output, or native setup change
  - remove stale statements instead of letting docs drift behind the implementation
- Documentation must describe the current tool clearly and soberly:
  - explain how to use it, what it supports today, and what remains unsupported
  - write as operator/developer documentation for a supported tool, not as scratch notes, roadmap fragments, or speculative text
  - do not describe experimental or partial work as if it were production-ready
