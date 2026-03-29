# Licensing

This document is an engineering-oriented summary, not legal advice.

## Software License

This repository is licensed under `AGPL-3.0-only`.

The project is AGPL-oriented because Essentia is used as the analysis backend. The repository does not attempt to bypass, isolate, or weaken the AGPL implications of that dependency.

Relevant repository files:

- `LICENSE`
- `THIRD_PARTY.md`
- `native/essentia-wrapper/`

## Essentia Relationship

- Essentia is the intended native analysis backend
- the Rust workspace orchestrates analysis around it
- the native wrapper is a narrow bridge, not a license-avoidance boundary
- the repository does not attempt to treat the native boundary as a way to weaken AGPL obligations

## Distribution And Network Use

From an engineering-compliance perspective:

- distributing the software should be treated as AGPL-governed
- exposing the software over a network should also be treated as AGPL-relevant
- this repository does not document or endorse any strategy for weakening those obligations

## Output Data

Generated JSONL feature data is distinct from the software itself.

- it does not contain the repository source code
- it does not embed Essentia implementation code
- it is not automatically treated as AGPL solely because this software produced it

That does not remove other rights considerations:

- rights in the original audio material still apply
- distribution or reuse of audio-derived data may still depend on the source material and the surrounding legal context

In short: generated feature data is not automatically subject to AGPL and can be used independently, subject to rights on the original audio material.

## Documentation Posture

This repository documents licensing conservatively:

- software licensing is described in AGPL-oriented terms
- output data is documented separately from the software license
- the documentation does not promise any strategy for isolating Essentia from the rest of the tool
