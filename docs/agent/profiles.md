
# Profiles

## minimal
- fastest
- core descriptors only
- no vector features

## default
- balanced
- includes mfcc
- richer spectral + tonal + rhythm

## research
- extended descriptors
- includes band-based + gfcc + spectral_peaks
- higher computational cost

## Rules
- spectral_peaks disabled unless research
- frame-level disabled by default
