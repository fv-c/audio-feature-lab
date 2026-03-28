
# Features Vocabulary (CONTROLLED)

## Spectral (scalar)
centroid, spread, skewness, kurtosis, rolloff, flux, flatness,
crest, energy, entropy, complexity, contrast, hfc,
strong_peak, dissonance, inharmonicity

## Spectral (vector)
mfcc, bark_bands, mel_bands, erb_bands, gfcc, spectral_peaks

## Temporal
zcr, rms, peak, envelope, dynamic_range

## Rhythm
onset_rate, onset_strength, tempo, beat_period, inter_onset_interval

## Tonal
hpcp, chroma, key_strength, tuning_frequency

## Dynamics
loudness, loudness_ebu, dynamic_complexity

## Metadata
duration, silence_ratio, active_ratio

## RULES

- lowercase snake_case only
- no aliases
- no implicit transformations
- unsupported features must be omitted
