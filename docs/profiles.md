# Profiles

The repository ships three built-in profile configs:

- `configs/minimal.toml`
- `configs/default.toml`
- `configs/research.toml`

These files are the current operational defaults for the CLI and benchmark suite.

All shipped configs currently include:

- `[backend].name = "essentia"`

## Shared Current Behavior

- configuration is loaded from TOML into strongly typed Rust structs
- profile names are validated
- feature families and feature names are validated against the controlled vocabulary
- aggregation statistics are currently implemented as `mean`
- frame-level extraction is disabled by default in all shipped configs
- `[performance].workers` is currently `1` in all shipped configs

The worker default is conservative on purpose. Current measured local native runs showed regression at higher counts.

## minimal

Purpose:

- lowest-cost baseline profile
- useful for fast corpus passes and pipeline validation

Current shipped config:

- focuses on a small set of spectral, temporal, dynamics, and metadata features
- keeps frame-level disabled
- uses `mean` aggregation only

Important caveat:

- the current `minimal.toml` is the repository's operational profile file, even where it is slightly more conservative or narrower than the broader ideal described in `docs/agent/profiles.md`

## default

Purpose:

- balanced general-use corpus analysis
- includes vector features such as `mfcc`

Current shipped config:

- enables spectral, rhythm, tonal, dynamics, and metadata families
- keeps frame-level disabled
- uses `mean` aggregation only

## research

Purpose:

- heavier exploratory profile for richer descriptor requests

Current shipped config:

- extends the requested spectral set with band features, `gfcc`, and `spectral_peaks`
- keeps frame-level disabled by default
- uses `mean` aggregation only

Important caveat:

- the current native backend does not emit every requested research descriptor yet
- unsupported descriptors remain omitted with warnings instead of being approximated

## Backend Coverage Caveat

The profile vocabulary is broader than the currently implemented native descriptor coverage. In practice:

- requesting a feature does not force the backend to fabricate it
- unsupported requested features remain absent from the output
- warnings document these omissions

This is intentional and preferable to renaming or approximating descriptors.

Selecting a different backend such as `mpeg7` is now supported by config and pipeline dispatch, but shipped profile files remain anchored to `essentia` until a real MPEG-7 native implementation exists.
