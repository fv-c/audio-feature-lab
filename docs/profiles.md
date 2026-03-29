# Profiles

The repository ships three profile configs:

- `configs/minimal.toml`
- `configs/default.toml`
- `configs/research.toml`

These are the operational defaults for the current public Essentia backend. They are intentionally constrained to descriptors the wrapper maps today.

## Shared Current Behavior

- `[backend].name = "essentia"` in all shipped configs
- `aggregation.statistics = ["mean"]` in all shipped configs
- `frame_level = false` in all shipped configs
- `[performance].workers = 1` in all shipped configs

The worker default is conservative on purpose. Current measured native runs regressed at higher worker counts on the validated local macOS path.

## minimal

Purpose:

- fastest operational baseline
- low-cost corpus passes
- no vector features

Current shipped features:

- `spectral`: `centroid`, `rolloff`, `flux`, `energy`
- `temporal`: `zcr`, `rms`
- `metadata`: `duration`, `silence_ratio`, `active_ratio`

## default

Purpose:

- balanced operational profile
- richer spectral coverage plus supported temporal, rhythm, tonal, dynamics, and metadata features

Current shipped features:

- `spectral`: `centroid`, `spread`, `skewness`, `kurtosis`, `rolloff`, `flux`, `energy`, `entropy`, `complexity`, `hfc`, `strong_peak`, `dissonance`, `mfcc`
- `temporal`: `zcr`, `rms`, `dynamic_range`
- `rhythm`: `onset_rate`, `tempo`, `beat_period`
- `tonal`: `hpcp`, `chroma`, `key_strength`, `tuning_frequency`
- `dynamics`: `loudness`, `dynamic_complexity`
- `metadata`: `duration`, `silence_ratio`, `active_ratio`

## research

Purpose:

- heaviest shipped profile on the current backend
- extends `default` with supported band features and additional heavy descriptors

Current shipped features:

- all `default` features, plus:
- `spectral`: `bark_bands`, `mel_bands`, `erb_bands`, `gfcc`
- `dynamics`: `loudness_ebu`

## Deferred Descriptor Coverage

These controlled-vocabulary names remain explicitly deferred for the public Essentia backend and are not part of the shipped configs:

- `spectral`: `flatness`, `crest`, `contrast`, `inharmonicity`, `spectral_peaks`
- `temporal`: `peak`, `envelope`
- `rhythm`: `onset_strength`, `inter_onset_interval`

They are backlog, not operational product surface. Operator configs that request them should be rejected during validation instead of being treated as normal profile choices.

## Frame-Level Note

Frame-level extraction is supported only for a subset of descriptors and is disabled by default in all shipped configs.

Use `audio-feature-lab backend-info` to inspect the current frame-level-capable subset exposed by the backend.
