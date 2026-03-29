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
- the config and record model accept the full allowed aggregation statistic set
- frame-level extraction is disabled by default in all shipped configs
- `[performance].workers` is currently `1` in all shipped configs

The worker default is conservative on purpose. Current measured local native runs showed regression at higher counts.

Important operational caveat:

- the current Essentia backend still accepts only `aggregation.statistics = ["mean"]`
- the shipped configs are therefore the safe operational defaults for real analysis today

## minimal

Purpose:

- lowest-cost baseline profile
- useful for fast corpus passes and pipeline validation

Current shipped config:

- focuses on a small set of spectral, temporal, dynamics, and metadata features
- enabled features:
  - `centroid`
  - `rolloff`
  - `flux`
  - `flatness`
  - `zcr`
  - `rms`
  - `loudness`
  - `duration`
  - `silence_ratio`
  - `active_ratio`
- keeps frame-level disabled
- ships with `mean` aggregation only

Important caveat:

- the current `minimal.toml` is the repository's operational profile file, even where it is slightly more conservative or narrower than the broader ideal described in `docs/agent/profiles.md`

## default

Purpose:

- balanced general-use corpus analysis
- includes vector features such as `mfcc`

Current shipped config:

- enables spectral, rhythm, tonal, dynamics, and metadata families
- enabled features:
  - `centroid`
  - `spread`
  - `rolloff`
  - `flux`
  - `flatness`
  - `entropy`
  - `hfc`
  - `mfcc`
  - `tempo`
  - `beat_period`
  - `onset_strength`
  - `hpcp`
  - `chroma`
  - `key_strength`
  - `tuning_frequency`
  - `loudness`
  - `dynamic_complexity`
  - `duration`
  - `silence_ratio`
  - `active_ratio`
- keeps frame-level disabled
- ships with `mean` aggregation only

## research

Purpose:

- heavier exploratory profile for richer descriptor requests

Current shipped config:

- extends the requested spectral set with band features, `gfcc`, and `spectral_peaks`
- enabled features:
  - `centroid`
  - `spread`
  - `rolloff`
  - `flux`
  - `flatness`
  - `entropy`
  - `contrast`
  - `hfc`
  - `dissonance`
  - `inharmonicity`
  - `mfcc`
  - `bark_bands`
  - `mel_bands`
  - `erb_bands`
  - `gfcc`
  - `spectral_peaks`
  - `tempo`
  - `beat_period`
  - `onset_strength`
  - `hpcp`
  - `chroma`
  - `key_strength`
  - `tuning_frequency`
  - `loudness`
  - `dynamic_complexity`
  - `duration`
  - `silence_ratio`
  - `active_ratio`
- keeps frame-level disabled by default
- ships with `mean` aggregation only

Important caveat:

- the current native backend does not emit every requested research descriptor yet
- unsupported descriptors remain omitted with warnings instead of being approximated

## Backend Coverage Caveat

The profile vocabulary is broader than the currently implemented native descriptor coverage. In practice:

- requesting a feature does not force the backend to fabricate it
- unsupported requested features remain absent from the output
- warnings document these omissions

This is intentional and preferable to renaming or approximating descriptors.

The shipped profile files remain anchored to `essentia`, which is the only backend currently supported publicly.
