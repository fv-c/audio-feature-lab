use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub type OrderedJsonObject = BTreeMap<String, JsonValue>;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRecord {
    pub schema: FeatureSchema,
    pub file: FileBlock,
    pub audio: AudioBlock,
    pub analysis: AnalysisBlock,
    pub features: FeaturesBlock,
    pub aggregation: Aggregation,
    pub provenance: ProvenanceBlock,
    pub status: StatusBlock,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FeatureSchema {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileBlock {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioBlock {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisBlock {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceBlock {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatusBlock {
    #[serde(flatten)]
    pub fields: OrderedJsonObject,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeaturesBlock {
    pub spectral: BTreeMap<SpectralFeature, FeatureValue>,
    pub temporal: BTreeMap<TemporalFeature, f64>,
    pub rhythm: BTreeMap<RhythmFeature, f64>,
    pub tonal: BTreeMap<TonalFeature, FeatureValue>,
    pub dynamics: BTreeMap<DynamicsFeature, f64>,
    pub metadata: BTreeMap<MetadataFeature, f64>,
    pub frame_level: Option<FrameLevelFeatures>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameLevelFeatures {
    pub spectral: BTreeMap<SpectralFeature, FrameFeatureValue>,
    pub temporal: BTreeMap<TemporalFeature, Vec<f64>>,
    pub rhythm: BTreeMap<RhythmFeature, Vec<f64>>,
    pub tonal: BTreeMap<TonalFeature, FrameFeatureValue>,
    pub dynamics: BTreeMap<DynamicsFeature, Vec<f64>>,
    pub metadata: BTreeMap<MetadataFeature, Vec<f64>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Aggregation {
    pub spectral: BTreeMap<SpectralFeature, AggregatedFeature>,
    pub temporal: BTreeMap<TemporalFeature, ScalarStatistics>,
    pub rhythm: BTreeMap<RhythmFeature, ScalarStatistics>,
    pub tonal: BTreeMap<TonalFeature, AggregatedFeature>,
    pub dynamics: BTreeMap<DynamicsFeature, ScalarStatistics>,
    pub metadata: BTreeMap<MetadataFeature, ScalarStatistics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeatureValue {
    Scalar(f64),
    Vector(Vec<f64>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FrameFeatureValue {
    Scalar(Vec<f64>),
    Vector(Vec<Vec<f64>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AggregatedFeature {
    Scalar(ScalarStatistics),
    Vector(VectorStatistics),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScalarStatistics {
    pub mean: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VectorStatistics {
    pub mean: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpectralFeature {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalFeature {
    Zcr,
    Rms,
    Peak,
    Envelope,
    DynamicRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RhythmFeature {
    OnsetRate,
    OnsetStrength,
    Tempo,
    BeatPeriod,
    InterOnsetInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TonalFeature {
    Hpcp,
    Chroma,
    KeyStrength,
    TuningFrequency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicsFeature {
    Loudness,
    LoudnessEbu,
    DynamicComplexity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataFeature {
    Duration,
    SilenceRatio,
    ActiveRatio,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        AggregatedFeature, Aggregation, AnalysisBlock, AnalysisRecord, AudioBlock, DynamicsFeature,
        FeatureSchema, FeatureValue, FeaturesBlock, FileBlock, FrameFeatureValue,
        FrameLevelFeatures, MetadataFeature, ProvenanceBlock, RhythmFeature, ScalarStatistics,
        SpectralFeature, StatusBlock, TemporalFeature, TonalFeature, VectorStatistics,
    };

    #[test]
    fn serializes_top_level_blocks_in_spec_order() {
        let record = AnalysisRecord::default();
        let serialized = serde_json::to_string(&record).expect("record should serialize");

        let keys = [
            "\"schema\"",
            "\"file\"",
            "\"audio\"",
            "\"analysis\"",
            "\"features\"",
            "\"aggregation\"",
            "\"provenance\"",
            "\"status\"",
        ];

        let mut previous_index = 0;
        for key in keys {
            let index = serialized
                .find(key)
                .unwrap_or_else(|| panic!("missing top-level key {key}"));
            assert!(index >= previous_index, "top-level keys are out of order");
            previous_index = index;
        }
    }

    #[test]
    fn includes_all_mandatory_blocks_even_when_empty() {
        let record = AnalysisRecord::default();
        let value = serde_json::to_value(record).expect("record should serialize");
        let object = value.as_object().expect("top-level JSON object");

        assert_eq!(object.len(), 8);
        for key in [
            "schema",
            "file",
            "audio",
            "analysis",
            "features",
            "aggregation",
            "provenance",
            "status",
        ] {
            assert!(object.contains_key(key), "missing top-level block {key}");
        }
    }

    #[test]
    fn preserves_nested_aggregation_hierarchy_for_scalar_and_vector_features() {
        let mut aggregation = Aggregation::default();
        aggregation.spectral.insert(
            SpectralFeature::Centroid,
            AggregatedFeature::Scalar(ScalarStatistics { mean: Some(123.5) }),
        );
        aggregation.spectral.insert(
            SpectralFeature::Mfcc,
            AggregatedFeature::Vector(VectorStatistics {
                mean: Some(vec![1.0, 2.0, 3.0]),
            }),
        );

        let record = AnalysisRecord {
            aggregation,
            ..AnalysisRecord::default()
        };

        let value = serde_json::to_value(record).expect("record should serialize");
        assert_eq!(
            value["aggregation"]["spectral"]["centroid"]["mean"],
            json!(123.5)
        );
        assert_eq!(
            value["aggregation"]["spectral"]["mfcc"]["mean"],
            json!([1.0, 2.0, 3.0])
        );
        assert!(value["aggregation"].get("spectral.centroid.mean").is_none());
    }

    #[test]
    fn frame_level_defaults_to_null_when_disabled() {
        let record = AnalysisRecord::default();
        let value = serde_json::to_value(record).expect("record should serialize");

        assert_eq!(value["features"]["frame_level"], Value::Null);
    }

    #[test]
    fn frame_level_serializes_when_enabled() {
        let mut frame_level = FrameLevelFeatures::default();
        frame_level.spectral.insert(
            SpectralFeature::Flux,
            FrameFeatureValue::Scalar(vec![0.1, 0.2]),
        );
        frame_level.tonal.insert(
            TonalFeature::Hpcp,
            FrameFeatureValue::Vector(vec![vec![0.3, 0.7], vec![0.4, 0.6]]),
        );

        let record = AnalysisRecord {
            features: FeaturesBlock {
                frame_level: Some(frame_level),
                ..FeaturesBlock::default()
            },
            ..AnalysisRecord::default()
        };

        let value = serde_json::to_value(record).expect("record should serialize");
        assert_eq!(
            value["features"]["frame_level"]["spectral"]["flux"],
            json!([0.1, 0.2])
        );
        assert_eq!(
            value["features"]["frame_level"]["tonal"]["hpcp"],
            json!([[0.3, 0.7], [0.4, 0.6]])
        );
    }

    #[test]
    fn serializes_feature_blocks_with_controlled_vocabulary_names() {
        let mut features = FeaturesBlock::default();
        features.spectral.insert(
            SpectralFeature::SpectralPeaks,
            FeatureValue::Vector(vec![110.0, 220.0]),
        );
        features
            .temporal
            .insert(TemporalFeature::DynamicRange, 12.5);
        features
            .rhythm
            .insert(RhythmFeature::InterOnsetInterval, 0.48);
        features
            .dynamics
            .insert(DynamicsFeature::LoudnessEbu, -14.0);
        features
            .metadata
            .insert(MetadataFeature::SilenceRatio, 0.08);

        let record = AnalysisRecord {
            features,
            ..AnalysisRecord::default()
        };

        let value = serde_json::to_value(record).expect("record should serialize");
        assert_eq!(
            value["features"]["spectral"]["spectral_peaks"],
            json!([110.0, 220.0])
        );
        assert_eq!(value["features"]["temporal"]["dynamic_range"], json!(12.5));
        assert_eq!(
            value["features"]["rhythm"]["inter_onset_interval"],
            json!(0.48)
        );
        assert_eq!(value["features"]["dynamics"]["loudness_ebu"], json!(-14.0));
        assert_eq!(value["features"]["metadata"]["silence_ratio"], json!(0.08));
    }

    #[test]
    fn serializes_deterministically_for_equivalent_records() {
        let record = sample_record();

        let first = serde_json::to_string(&record).expect("record should serialize");
        let second = serde_json::to_string(&record).expect("record should serialize");

        assert_eq!(first, second);
    }

    fn sample_record() -> AnalysisRecord {
        let mut schema = FeatureSchema::default();
        schema.fields.insert("version".to_string(), json!("1.0.0"));

        let mut file = FileBlock::default();
        file.fields
            .insert("path".to_string(), json!("audio/example.wav"));

        let mut audio = AudioBlock::default();
        audio.fields.insert("sample_rate".to_string(), json!(44100));

        let mut analysis = AnalysisBlock::default();
        analysis
            .fields
            .insert("profile".to_string(), json!("default"));

        let mut provenance = ProvenanceBlock::default();
        provenance
            .fields
            .insert("backend".to_string(), json!("essentia"));

        let mut status = StatusBlock::default();
        status.fields.insert("code".to_string(), json!("ok"));

        let mut features = FeaturesBlock::default();
        features.spectral.insert(
            SpectralFeature::Mfcc,
            FeatureValue::Vector(vec![0.1, 0.2, 0.3]),
        );
        features
            .tonal
            .insert(TonalFeature::KeyStrength, FeatureValue::Scalar(0.91));

        let mut aggregation = Aggregation::default();
        aggregation.spectral.insert(
            SpectralFeature::Mfcc,
            AggregatedFeature::Vector(VectorStatistics {
                mean: Some(vec![0.1, 0.2, 0.3]),
            }),
        );
        aggregation.tonal.insert(
            TonalFeature::KeyStrength,
            AggregatedFeature::Scalar(ScalarStatistics { mean: Some(0.91) }),
        );

        AnalysisRecord {
            schema,
            file,
            audio,
            analysis,
            features,
            aggregation,
            provenance,
            status,
        }
    }
}
