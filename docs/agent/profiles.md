
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
- within the target profile design, spectral_peaks belongs only to research
- frame-level disabled by default
- shipped configs, CLI help, and user-facing docs must stay within the currently implemented public backend capability
- when the backend is narrower than this target profile description, narrow the public profile surface and document the missing descriptors as future work
