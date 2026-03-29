use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Minimal,
    Default,
    Research,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Default => "default",
            Self::Research => "research",
        }
    }

    pub fn example(self) -> &'static str {
        match self {
            Self::Minimal => include_str!("../../../configs/minimal.toml"),
            Self::Default => include_str!("../../../configs/default.toml"),
            Self::Research => include_str!("../../../configs/research.toml"),
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "default" => Ok(Self::Default),
            "research" => Ok(Self::Research),
            _ => Err(ConfigError::UnknownProfile(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendName {
    Essentia,
}

impl BackendName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Essentia => "essentia",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "essentia" => Ok(Self::Essentia),
            _ => Err(ConfigError::UnknownBackend(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureFamily {
    Spectral,
    Temporal,
    Rhythm,
    Tonal,
    Dynamics,
    Metadata,
}

impl FeatureFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spectral => "spectral",
            Self::Temporal => "temporal",
            Self::Rhythm => "rhythm",
            Self::Tonal => "tonal",
            Self::Dynamics => "dynamics",
            Self::Metadata => "metadata",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "spectral" => Ok(Self::Spectral),
            "temporal" => Ok(Self::Temporal),
            "rhythm" => Ok(Self::Rhythm),
            "tonal" => Ok(Self::Tonal),
            "dynamics" => Ok(Self::Dynamics),
            "metadata" => Ok(Self::Metadata),
            _ => Err(ConfigError::UnknownFeatureFamily(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureName {
    Centroid,
    Spread,
    Skewness,
    Kurtosis,
    Rolloff,
    Flux,
    Flatness,
    Crest,
    Energy,
    Entropy,
    Complexity,
    Contrast,
    Hfc,
    StrongPeak,
    Dissonance,
    Inharmonicity,
    Mfcc,
    BarkBands,
    MelBands,
    ErbBands,
    Gfcc,
    SpectralPeaks,
    Zcr,
    Rms,
    Peak,
    Envelope,
    DynamicRange,
    OnsetRate,
    OnsetStrength,
    Tempo,
    BeatPeriod,
    InterOnsetInterval,
    Hpcp,
    Chroma,
    KeyStrength,
    TuningFrequency,
    Loudness,
    LoudnessEbu,
    DynamicComplexity,
    Duration,
    SilenceRatio,
    ActiveRatio,
}

impl FeatureName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Centroid => "centroid",
            Self::Spread => "spread",
            Self::Skewness => "skewness",
            Self::Kurtosis => "kurtosis",
            Self::Rolloff => "rolloff",
            Self::Flux => "flux",
            Self::Flatness => "flatness",
            Self::Crest => "crest",
            Self::Energy => "energy",
            Self::Entropy => "entropy",
            Self::Complexity => "complexity",
            Self::Contrast => "contrast",
            Self::Hfc => "hfc",
            Self::StrongPeak => "strong_peak",
            Self::Dissonance => "dissonance",
            Self::Inharmonicity => "inharmonicity",
            Self::Mfcc => "mfcc",
            Self::BarkBands => "bark_bands",
            Self::MelBands => "mel_bands",
            Self::ErbBands => "erb_bands",
            Self::Gfcc => "gfcc",
            Self::SpectralPeaks => "spectral_peaks",
            Self::Zcr => "zcr",
            Self::Rms => "rms",
            Self::Peak => "peak",
            Self::Envelope => "envelope",
            Self::DynamicRange => "dynamic_range",
            Self::OnsetRate => "onset_rate",
            Self::OnsetStrength => "onset_strength",
            Self::Tempo => "tempo",
            Self::BeatPeriod => "beat_period",
            Self::InterOnsetInterval => "inter_onset_interval",
            Self::Hpcp => "hpcp",
            Self::Chroma => "chroma",
            Self::KeyStrength => "key_strength",
            Self::TuningFrequency => "tuning_frequency",
            Self::Loudness => "loudness",
            Self::LoudnessEbu => "loudness_ebu",
            Self::DynamicComplexity => "dynamic_complexity",
            Self::Duration => "duration",
            Self::SilenceRatio => "silence_ratio",
            Self::ActiveRatio => "active_ratio",
        }
    }

    pub fn family(self) -> FeatureFamily {
        match self {
            Self::Centroid
            | Self::Spread
            | Self::Skewness
            | Self::Kurtosis
            | Self::Rolloff
            | Self::Flux
            | Self::Flatness
            | Self::Crest
            | Self::Energy
            | Self::Entropy
            | Self::Complexity
            | Self::Contrast
            | Self::Hfc
            | Self::StrongPeak
            | Self::Dissonance
            | Self::Inharmonicity
            | Self::Mfcc
            | Self::BarkBands
            | Self::MelBands
            | Self::ErbBands
            | Self::Gfcc
            | Self::SpectralPeaks => FeatureFamily::Spectral,
            Self::Zcr | Self::Rms | Self::Peak | Self::Envelope | Self::DynamicRange => {
                FeatureFamily::Temporal
            }
            Self::OnsetRate
            | Self::OnsetStrength
            | Self::Tempo
            | Self::BeatPeriod
            | Self::InterOnsetInterval => FeatureFamily::Rhythm,
            Self::Hpcp | Self::Chroma | Self::KeyStrength | Self::TuningFrequency => {
                FeatureFamily::Tonal
            }
            Self::Loudness | Self::LoudnessEbu | Self::DynamicComplexity => FeatureFamily::Dynamics,
            Self::Duration | Self::SilenceRatio | Self::ActiveRatio => FeatureFamily::Metadata,
        }
    }

    pub fn is_vector(self) -> bool {
        matches!(
            self,
            Self::Mfcc
                | Self::BarkBands
                | Self::MelBands
                | Self::ErbBands
                | Self::Gfcc
                | Self::SpectralPeaks
                | Self::Hpcp
                | Self::Chroma
        )
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "centroid" => Ok(Self::Centroid),
            "spread" => Ok(Self::Spread),
            "skewness" => Ok(Self::Skewness),
            "kurtosis" => Ok(Self::Kurtosis),
            "rolloff" => Ok(Self::Rolloff),
            "flux" => Ok(Self::Flux),
            "flatness" => Ok(Self::Flatness),
            "crest" => Ok(Self::Crest),
            "energy" => Ok(Self::Energy),
            "entropy" => Ok(Self::Entropy),
            "complexity" => Ok(Self::Complexity),
            "contrast" => Ok(Self::Contrast),
            "hfc" => Ok(Self::Hfc),
            "strong_peak" => Ok(Self::StrongPeak),
            "dissonance" => Ok(Self::Dissonance),
            "inharmonicity" => Ok(Self::Inharmonicity),
            "mfcc" => Ok(Self::Mfcc),
            "bark_bands" => Ok(Self::BarkBands),
            "mel_bands" => Ok(Self::MelBands),
            "erb_bands" => Ok(Self::ErbBands),
            "gfcc" => Ok(Self::Gfcc),
            "spectral_peaks" => Ok(Self::SpectralPeaks),
            "zcr" => Ok(Self::Zcr),
            "rms" => Ok(Self::Rms),
            "peak" => Ok(Self::Peak),
            "envelope" => Ok(Self::Envelope),
            "dynamic_range" => Ok(Self::DynamicRange),
            "onset_rate" => Ok(Self::OnsetRate),
            "onset_strength" => Ok(Self::OnsetStrength),
            "tempo" => Ok(Self::Tempo),
            "beat_period" => Ok(Self::BeatPeriod),
            "inter_onset_interval" => Ok(Self::InterOnsetInterval),
            "hpcp" => Ok(Self::Hpcp),
            "chroma" => Ok(Self::Chroma),
            "key_strength" => Ok(Self::KeyStrength),
            "tuning_frequency" => Ok(Self::TuningFrequency),
            "loudness" => Ok(Self::Loudness),
            "loudness_ebu" => Ok(Self::LoudnessEbu),
            "dynamic_complexity" => Ok(Self::DynamicComplexity),
            "duration" => Ok(Self::Duration),
            "silence_ratio" => Ok(Self::SilenceRatio),
            "active_ratio" => Ok(Self::ActiveRatio),
            _ => Err(ConfigError::UnknownFeature(value.to_string())),
        }
    }
}

impl BackendName {
    pub fn declared_exact_features(self) -> &'static [FeatureName] {
        let _ = self;
        &[]
    }

    pub fn supports_feature(self, feature: FeatureName) -> bool {
        let _ = (self, feature);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregationStatistic {
    Mean,
    Std,
    Min,
    Max,
    Median,
    P10,
    P25,
    P75,
    P90,
}

impl AggregationStatistic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mean => "mean",
            Self::Std => "std",
            Self::Min => "min",
            Self::Max => "max",
            Self::Median => "median",
            Self::P10 => "p10",
            Self::P25 => "p25",
            Self::P75 => "p75",
            Self::P90 => "p90",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "mean" => Ok(Self::Mean),
            "std" => Ok(Self::Std),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "median" => Ok(Self::Median),
            "p10" => Ok(Self::P10),
            "p25" => Ok(Self::P25),
            "p75" => Ok(Self::P75),
            "p90" => Ok(Self::P90),
            _ => Err(ConfigError::UnknownAggregationStatistic(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeaturesConfig {
    pub families: Vec<FeatureFamily>,
    pub enabled: Vec<FeatureName>,
    pub frame_level: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregationConfig {
    pub statistics: Vec<AggregationStatistic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendConfig {
    pub name: BackendName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabConfig {
    pub profile: Profile,
    pub backend: BackendConfig,
    pub features: FeaturesConfig,
    pub aggregation: AggregationConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceConfig {
    pub workers: usize,
}

impl LabConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        match path {
            Some(path) => Self::from_path(path),
            None => Self::load_default(),
        }
    }

    pub fn load_default() -> Result<Self, ConfigError> {
        Self::from_profile(Profile::Default)
    }

    pub fn from_profile(profile: Profile) -> Result<Self, ConfigError> {
        let raw = parse_raw(profile.example())?;
        Self::validate_raw(raw, ResolveProfileDefaults::No)
    }

    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(|error| ConfigError::Io {
            path: path.to_path_buf(),
            error,
        })?;

        source.parse()
    }

    pub fn parse_str(source: &str) -> Result<Self, ConfigError> {
        let raw = parse_raw(source)?;
        Self::validate_raw(raw, ResolveProfileDefaults::Yes)
    }

    fn validate_raw(
        raw: RawLabConfig,
        resolve_defaults: ResolveProfileDefaults,
    ) -> Result<Self, ConfigError> {
        let profile = Profile::parse(&raw.profile)?;
        let base = match resolve_defaults {
            ResolveProfileDefaults::Yes
                if raw.backend.is_none() || raw.features.is_none() || raw.aggregation.is_none() =>
            {
                Some(Self::from_profile(profile)?)
            }
            ResolveProfileDefaults::Yes | ResolveProfileDefaults::No => None,
        };

        let backend = match (raw.backend, base.as_ref().map(|config| &config.backend)) {
            (Some(raw_backend), _) => validate_backend(raw_backend)?,
            (None, Some(backend)) => backend.clone(),
            (None, None) => return Err(ConfigError::MissingSection("backend")),
        };

        let features = match (raw.features, base.as_ref().map(|config| &config.features)) {
            (Some(raw_features), _) => validate_features(raw_features, profile)?,
            (None, Some(features)) => features.clone(),
            (None, None) => return Err(ConfigError::MissingSection("features")),
        };

        validate_profile_constraints(profile, &features)?;
        validate_backend_constraints(backend.name, &features)?;

        let aggregation = match (
            raw.aggregation,
            base.as_ref().map(|config| &config.aggregation),
        ) {
            (Some(raw_aggregation), _) => validate_aggregation(raw_aggregation)?,
            (None, Some(aggregation)) => aggregation.clone(),
            (None, None) => return Err(ConfigError::MissingSection("aggregation")),
        };

        let performance = match (
            raw.performance,
            base.as_ref().map(|config| &config.performance),
        ) {
            (Some(raw_performance), _) => validate_performance(raw_performance)?,
            (None, Some(performance)) => performance.clone(),
            (None, None) => PerformanceConfig { workers: 1 },
        };

        Ok(Self {
            profile,
            backend,
            features,
            aggregation,
            performance,
        })
    }
}

impl std::str::FromStr for LabConfig {
    type Err = ConfigError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Self::parse_str(source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolveProfileDefaults {
    Yes,
    No,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLabConfig {
    profile: String,
    backend: Option<RawBackendConfig>,
    features: Option<RawFeaturesConfig>,
    aggregation: Option<RawAggregationConfig>,
    performance: Option<RawPerformanceConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBackendConfig {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFeaturesConfig {
    families: Vec<String>,
    enabled: Vec<String>,
    frame_level: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAggregationConfig {
    statistics: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPerformanceConfig {
    workers: usize,
}

fn parse_raw(source: &str) -> Result<RawLabConfig, ConfigError> {
    toml::from_str(source).map_err(ConfigError::TomlParse)
}

fn validate_backend(raw: RawBackendConfig) -> Result<BackendConfig, ConfigError> {
    Ok(BackendConfig {
        name: BackendName::parse(&raw.name)?,
    })
}

fn validate_features(
    raw: RawFeaturesConfig,
    profile: Profile,
) -> Result<FeaturesConfig, ConfigError> {
    let families = parse_unique_values(
        raw.families,
        "features.families",
        FeatureFamily::parse,
        FeatureFamily::as_str,
    )?;
    let enabled = parse_unique_values(
        raw.enabled,
        "features.enabled",
        FeatureName::parse,
        FeatureName::as_str,
    )?;

    let family_set = families.iter().copied().collect::<BTreeSet<_>>();
    for feature in &enabled {
        let family = feature.family();
        if !family_set.contains(&family) {
            return Err(ConfigError::FeatureOutsideSelectedFamilies {
                feature: feature.as_str().to_string(),
                family: family.as_str().to_string(),
            });
        }
    }

    if profile != Profile::Research && enabled.contains(&FeatureName::SpectralPeaks) {
        return Err(ConfigError::ProfileConstraint {
            profile,
            message: "spectral_peaks is only allowed in the research profile".to_string(),
        });
    }

    Ok(FeaturesConfig {
        families,
        enabled,
        frame_level: raw.frame_level.unwrap_or(false),
    })
}

fn validate_aggregation(raw: RawAggregationConfig) -> Result<AggregationConfig, ConfigError> {
    let statistics = parse_unique_values(
        raw.statistics,
        "aggregation.statistics",
        AggregationStatistic::parse,
        AggregationStatistic::as_str,
    )?;

    if statistics.is_empty() {
        return Err(ConfigError::EmptySelection("aggregation.statistics"));
    }

    Ok(AggregationConfig { statistics })
}

fn validate_performance(raw: RawPerformanceConfig) -> Result<PerformanceConfig, ConfigError> {
    if raw.workers == 0 {
        return Err(ConfigError::InvalidWorkers(0));
    }

    Ok(PerformanceConfig {
        workers: raw.workers,
    })
}

fn validate_profile_constraints(
    profile: Profile,
    features: &FeaturesConfig,
) -> Result<(), ConfigError> {
    match profile {
        Profile::Minimal => {
            if let Some(feature) = features.enabled.iter().find(|feature| feature.is_vector()) {
                return Err(ConfigError::ProfileConstraint {
                    profile,
                    message: format!(
                        "minimal profile cannot enable vector feature `{}`",
                        feature.as_str()
                    ),
                });
            }
        }
        Profile::Default => {
            if !features.enabled.contains(&FeatureName::Mfcc) {
                return Err(ConfigError::ProfileConstraint {
                    profile,
                    message: "default profile must include `mfcc`".to_string(),
                });
            }

            for family in [
                FeatureFamily::Spectral,
                FeatureFamily::Rhythm,
                FeatureFamily::Tonal,
            ] {
                if !features.families.contains(&family) {
                    return Err(ConfigError::ProfileConstraint {
                        profile,
                        message: format!(
                            "default profile must include the `{}` feature family",
                            family.as_str()
                        ),
                    });
                }
            }
        }
        Profile::Research => {
            let has_band_feature = features.enabled.iter().any(|feature| {
                matches!(
                    feature,
                    FeatureName::BarkBands | FeatureName::MelBands | FeatureName::ErbBands
                )
            });

            if !has_band_feature {
                return Err(ConfigError::ProfileConstraint {
                    profile,
                    message: "research profile must include at least one band-based feature".into(),
                });
            }

            for feature in [FeatureName::Gfcc, FeatureName::SpectralPeaks] {
                if !features.enabled.contains(&feature) {
                    return Err(ConfigError::ProfileConstraint {
                        profile,
                        message: format!("research profile must include `{}`", feature.as_str()),
                    });
                }
            }
        }
    }

    Ok(())
}

fn validate_backend_constraints(
    backend: BackendName,
    features: &FeaturesConfig,
) -> Result<(), ConfigError> {
    if let Some(feature) = features
        .enabled
        .iter()
        .find(|feature| !backend.supports_feature(**feature))
    {
        return Err(ConfigError::FeatureUnsupportedByBackend {
            backend,
            feature: feature.as_str().to_string(),
        });
    }

    Ok(())
}

fn parse_unique_values<T, ParseFn, NameFn>(
    values: Vec<String>,
    field: &'static str,
    parse: ParseFn,
    name: NameFn,
) -> Result<Vec<T>, ConfigError>
where
    T: Copy + Ord,
    ParseFn: Fn(&str) -> Result<T, ConfigError>,
    NameFn: Fn(T) -> &'static str,
{
    if values.is_empty() {
        return Err(ConfigError::EmptySelection(field));
    }

    let mut seen = BTreeSet::new();
    let mut parsed = Vec::with_capacity(values.len());

    for value in values {
        let trimmed = value.trim();
        let parsed_value = parse(trimmed)?;
        let canonical_name = name(parsed_value);

        if !seen.insert(canonical_name) {
            return Err(ConfigError::DuplicateValue {
                field,
                value: canonical_name.to_string(),
            });
        }

        parsed.push(parsed_value);
    }

    Ok(parsed)
}

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    TomlParse(toml::de::Error),
    MissingSection(&'static str),
    EmptySelection(&'static str),
    DuplicateValue {
        field: &'static str,
        value: String,
    },
    UnknownProfile(String),
    UnknownBackend(String),
    UnknownFeatureFamily(String),
    UnknownFeature(String),
    UnknownAggregationStatistic(String),
    FeatureOutsideSelectedFamilies {
        feature: String,
        family: String,
    },
    FeatureUnsupportedByBackend {
        backend: BackendName,
        feature: String,
    },
    ProfileConstraint {
        profile: Profile,
        message: String,
    },
    InvalidWorkers(usize),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(f, "failed to read {}: {error}", path.display())
            }
            Self::TomlParse(error) => write!(f, "invalid TOML configuration: {error}"),
            Self::MissingSection(section) => write!(f, "missing required section `{section}`"),
            Self::EmptySelection(field) => write!(f, "`{field}` cannot be empty"),
            Self::DuplicateValue { field, value } => {
                write!(f, "duplicate value `{value}` in `{field}`")
            }
            Self::UnknownProfile(profile) => write!(f, "unknown profile `{profile}`"),
            Self::UnknownBackend(backend) => write!(f, "unknown backend `{backend}`"),
            Self::UnknownFeatureFamily(family) => {
                write!(f, "unknown feature family `{family}`")
            }
            Self::UnknownFeature(feature) => write!(f, "unknown feature `{feature}`"),
            Self::UnknownAggregationStatistic(statistic) => {
                write!(f, "unknown aggregation statistic `{statistic}`")
            }
            Self::FeatureOutsideSelectedFamilies { feature, family } => write!(
                f,
                "feature `{feature}` requires the `{family}` family to be enabled"
            ),
            Self::FeatureUnsupportedByBackend { backend, feature } => write!(
                f,
                "feature `{feature}` is not currently supported by backend `{}`",
                backend.as_str()
            ),
            Self::ProfileConstraint { profile, message } => {
                write!(f, "profile `{}` is invalid: {message}", profile.as_str())
            }
            Self::InvalidWorkers(workers) => {
                write!(f, "`performance.workers` must be at least 1, got {workers}")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { error, .. } => Some(error),
            Self::TomlParse(error) => Some(error),
            Self::MissingSection(_)
            | Self::EmptySelection(_)
            | Self::DuplicateValue { .. }
            | Self::UnknownProfile(_)
            | Self::UnknownBackend(_)
            | Self::UnknownFeatureFamily(_)
            | Self::UnknownFeature(_)
            | Self::UnknownAggregationStatistic(_)
            | Self::FeatureOutsideSelectedFamilies { .. }
            | Self::FeatureUnsupportedByBackend { .. }
            | Self::ProfileConstraint { .. }
            | Self::InvalidWorkers(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AggregationStatistic, BackendName, ConfigError, FeatureFamily, FeatureName, LabConfig,
        Profile,
    };

    #[test]
    fn loads_default_config_when_no_path_is_provided() {
        let config = LabConfig::load(None).expect("default config should load");

        assert_eq!(config.profile, Profile::Default);
        assert_eq!(config.backend.name, BackendName::Essentia);
        assert!(config.features.enabled.contains(&FeatureName::Mfcc));
        assert_eq!(
            config.aggregation.statistics,
            vec![AggregationStatistic::Mean]
        );
        assert!(!config.features.frame_level);
        assert_eq!(config.performance.workers, 1);
    }

    #[test]
    fn loads_each_profile_example() {
        for profile in [Profile::Minimal, Profile::Default, Profile::Research] {
            let config = LabConfig::from_profile(profile).expect("profile example should load");
            assert_eq!(config.profile, profile);
            assert_eq!(config.backend.name, BackendName::Essentia);
            assert!(!config.features.families.is_empty());
            assert!(!config.features.enabled.is_empty());
            assert_eq!(
                config.aggregation.statistics,
                vec![AggregationStatistic::Mean]
            );
            assert_eq!(config.performance.workers, 1);
        }
    }

    #[test]
    fn shipped_default_profile_includes_temporal_family_and_features() {
        let config =
            LabConfig::from_profile(Profile::Default).expect("default profile should load");

        assert!(config.features.families.contains(&FeatureFamily::Temporal));
        assert!(config.features.enabled.contains(&FeatureName::Zcr));
        assert!(config.features.enabled.contains(&FeatureName::Rms));
        assert!(config.features.enabled.contains(&FeatureName::DynamicRange));
    }

    #[test]
    fn shipped_research_profile_includes_temporal_family_and_features() {
        let config =
            LabConfig::from_profile(Profile::Research).expect("research profile should load");

        assert!(config.features.families.contains(&FeatureFamily::Temporal));
        assert!(config.features.enabled.contains(&FeatureName::Zcr));
        assert!(config.features.enabled.contains(&FeatureName::Rms));
        assert!(config.features.enabled.contains(&FeatureName::Peak));
        assert!(config.features.enabled.contains(&FeatureName::Envelope));
        assert!(config.features.enabled.contains(&FeatureName::DynamicRange));
    }

    #[test]
    fn custom_config_can_inherit_profile_defaults() {
        let config = LabConfig::parse_str("profile = \"minimal\"").expect("minimal profile");

        assert_eq!(config.profile, Profile::Minimal);
        assert_eq!(config.backend.name, BackendName::Essentia);
        assert!(
            !config
                .features
                .enabled
                .iter()
                .any(|feature| feature.is_vector())
        );
        assert_eq!(
            config.aggregation.statistics,
            vec![AggregationStatistic::Mean]
        );
        assert_eq!(config.performance.workers, 1);
    }

    #[test]
    fn custom_feature_section_uses_default_aggregation_section() {
        let config = LabConfig::parse_str(
            r#"
profile = "default"

[features]
families = ["spectral", "rhythm", "tonal"]
enabled = ["mfcc", "tempo", "hpcp"]
"#,
        )
        .expect("default profile config should load");

        assert_eq!(config.profile, Profile::Default);
        assert_eq!(config.backend.name, BackendName::Essentia);
        assert_eq!(
            config.features.families,
            vec![
                FeatureFamily::Spectral,
                FeatureFamily::Rhythm,
                FeatureFamily::Tonal,
            ]
        );
        assert_eq!(
            config.aggregation.statistics,
            vec![AggregationStatistic::Mean]
        );
        assert!(!config.features.frame_level);
    }

    #[test]
    fn rejects_unknown_profile() {
        let error = LabConfig::parse_str("profile = \"fast\"").expect_err("invalid profile");

        assert_eq!(error.to_string(), "unknown profile `fast`");
    }

    #[test]
    fn rejects_unknown_backend() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[backend]
name = "future-dsp"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("invalid backend");

        assert_eq!(error.to_string(), "unknown backend `future-dsp`");
    }

    #[test]
    fn rejects_unknown_feature_family() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "pitch"]
enabled = ["centroid"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("invalid family");

        assert_eq!(error.to_string(), "unknown feature family `pitch`");
    }

    #[test]
    fn rejects_unknown_feature() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral"]
enabled = ["brightness"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("invalid feature");

        assert_eq!(error.to_string(), "unknown feature `brightness`");
    }

    #[test]
    fn rejects_feature_outside_selected_family() {
        let error = LabConfig::parse_str(
            r#"
profile = "default"

[features]
families = ["spectral", "rhythm", "tonal"]
enabled = ["mfcc", "tempo", "loudness"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("feature family mismatch");

        assert_eq!(
            error.to_string(),
            "feature `loudness` requires the `dynamics` family to be enabled"
        );
    }

    #[test]
    fn rejects_duplicate_statistics() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean", "mean"]
"#,
        )
        .expect_err("duplicate statistic");

        assert_eq!(
            error.to_string(),
            "duplicate value `mean` in `aggregation.statistics`"
        );
    }

    #[test]
    fn rejects_unsupported_aggregation_statistic() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["variance"]
"#,
        )
        .expect_err("unsupported statistic");

        assert_eq!(
            error.to_string(),
            "unknown aggregation statistic `variance`"
        );
    }

    #[test]
    fn parses_full_supported_aggregation_statistics() {
        let config = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean", "std", "min", "max", "median", "p10", "p25", "p75", "p90"]
"#,
        )
        .expect("supported statistics should parse");

        assert_eq!(
            config.aggregation.statistics,
            vec![
                AggregationStatistic::Mean,
                AggregationStatistic::Std,
                AggregationStatistic::Min,
                AggregationStatistic::Max,
                AggregationStatistic::Median,
                AggregationStatistic::P10,
                AggregationStatistic::P25,
                AggregationStatistic::P75,
                AggregationStatistic::P90,
            ]
        );
    }

    #[test]
    fn rejects_vector_features_in_minimal_profile() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["mfcc", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("minimal profile should reject vectors");

        assert_eq!(
            error.to_string(),
            "profile `minimal` is invalid: minimal profile cannot enable vector feature `mfcc`"
        );
    }

    #[test]
    fn rejects_default_profile_without_mfcc() {
        let error = LabConfig::parse_str(
            r#"
profile = "default"

[features]
families = ["spectral", "rhythm", "tonal"]
enabled = ["tempo", "hpcp"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("default profile should require mfcc");

        assert_eq!(
            error.to_string(),
            "profile `default` is invalid: default profile must include `mfcc`"
        );
    }

    #[test]
    fn rejects_spectral_peaks_outside_research() {
        let error = LabConfig::parse_str(
            r#"
profile = "default"

[features]
families = ["spectral", "rhythm", "tonal"]
enabled = ["mfcc", "tempo", "hpcp", "spectral_peaks"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("spectral_peaks should be restricted");

        assert_eq!(
            error.to_string(),
            "profile `default` is invalid: spectral_peaks is only allowed in the research profile"
        );
    }

    #[test]
    fn rejects_research_profile_without_required_extended_features() {
        let error = LabConfig::parse_str(
            r#"
profile = "research"

[features]
families = ["spectral", "temporal", "rhythm", "tonal", "dynamics", "metadata"]
enabled = ["mfcc", "tempo", "hpcp", "loudness", "duration"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("research profile should require extended features");

        assert_eq!(
            error.to_string(),
            "profile `research` is invalid: research profile must include at least one band-based feature"
        );
    }

    #[test]
    fn frame_level_defaults_to_false_when_omitted() {
        let config = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect("config should load");

        assert!(!config.features.frame_level);
        assert_eq!(config.performance.workers, 1);
    }

    #[test]
    fn parses_performance_workers_when_present() {
        let config = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean"]

[performance]
workers = 3
"#,
        )
        .expect("config should load");

        assert_eq!(config.performance.workers, 3);
    }

    #[test]
    fn rejects_zero_workers() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = ["spectral", "temporal", "dynamics", "metadata"]
enabled = ["centroid", "zcr", "loudness", "duration"]

[aggregation]
statistics = ["mean"]

[performance]
workers = 0
"#,
        )
        .expect_err("workers must be positive");

        assert_eq!(
            error.to_string(),
            "`performance.workers` must be at least 1, got 0"
        );
    }

    #[test]
    fn toml_errors_are_contextual() {
        let error = LabConfig::parse_str(
            r#"
profile = "minimal"

[features]
families = "spectral"
enabled = ["centroid"]

[aggregation]
statistics = ["mean"]
"#,
        )
        .expect_err("invalid TOML shape");

        match error {
            ConfigError::TomlParse(_) => {}
            other => panic!("expected TOML parse error, got {other}"),
        }
    }
}
