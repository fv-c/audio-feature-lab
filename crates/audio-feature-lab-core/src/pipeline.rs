use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

use audio_feature_lab_config::{BackendName, LabConfig};
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use crate::domain::{
    Aggregation, AnalysisBlock, AnalysisRecord, AudioBlock, FeatureSchema, FeaturesBlock,
    FileBlock, OrderedJsonObject, ProvenanceBlock, StatusBlock,
};
use crate::storage::{RecordSink, StorageError};
use crate::walker::{FileIdentity, WalkError, WalkedFile, Walker};

const SCHEMA_NAME: &str = "audio-feature-lab-record";
const SCHEMA_VERSION: &str = "1.0.0";
const TOOL_NAME: &str = "audio-feature-lab";

#[derive(Debug)]
pub struct Pipeline<B = NativeBackend> {
    config: LabConfig,
    config_json: String,
    walker: Walker,
    backend: B,
    backend_version: Option<String>,
}

impl Pipeline<NativeBackend> {
    pub fn with_configured_backend(
        config: LabConfig,
        walker: Walker,
    ) -> Result<Self, PipelineError> {
        let backend = NativeBackend::new(config.backend.name);
        Self::new(config, walker, backend)
    }
}

impl<B: Backend> Pipeline<B> {
    pub fn new(config: LabConfig, walker: Walker, backend: B) -> Result<Self, PipelineError> {
        let config_json = serialize_backend_config(&config)?;
        let backend_version = backend.backend_version().ok();

        Ok(Self {
            config,
            config_json,
            walker,
            backend,
            backend_version,
        })
    }

    pub fn config(&self) -> &LabConfig {
        &self.config
    }

    pub fn walker(&self) -> &Walker {
        &self.walker
    }

    pub fn process_file(&self, path: &Path) -> Result<AnalysisRecord, PipelineError> {
        let identity = FileIdentity::from_path(path).map_err(PipelineError::Walk)?;
        self.process_entry(path, None, identity)
    }

    pub fn process_batch<I, P, S>(
        &self,
        paths: I,
        sink: &mut S,
    ) -> Result<PipelineStats, PipelineError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
        S: RecordSink,
    {
        self.process_jobs(
            paths.into_iter().enumerate().map(|(index, path)| {
                let path = path.as_ref().to_path_buf();
                let identity = FileIdentity::from_path(&path).map_err(PipelineError::Walk)?;
                Ok(AnalysisJob {
                    index,
                    path,
                    relative_path: None,
                    identity,
                })
            }),
            sink,
        )
    }

    pub fn process_scan<S>(&self, root: &Path, sink: &mut S) -> Result<PipelineStats, PipelineError>
    where
        S: RecordSink,
    {
        self.process_jobs(
            self.walker
                .walk(root)
                .map_err(PipelineError::Walk)?
                .enumerate()
                .map(|(index, entry)| {
                    let walked_file = entry.map_err(PipelineError::Walk)?;
                    Ok(AnalysisJob {
                        index,
                        path: walked_file.path,
                        relative_path: Some(walked_file.relative_path),
                        identity: walked_file.identity,
                    })
                }),
            sink,
        )
    }

    pub fn process_walked_file(
        &self,
        walked_file: &WalkedFile,
    ) -> Result<AnalysisRecord, PipelineError> {
        self.process_entry(
            walked_file.path.as_path(),
            Some(walked_file.relative_path.as_path()),
            walked_file.identity.clone(),
        )
    }

    fn process_entry(
        &self,
        path: &Path,
        relative_path: Option<&Path>,
        identity: FileIdentity,
    ) -> Result<AnalysisRecord, PipelineError> {
        let (audio, features, aggregation, status) =
            match self.backend.analyze_file(path, &self.config_json) {
                Ok(response) => match serde_json::from_str::<BackendPayload>(&response) {
                    Ok(payload) => (
                        payload.audio,
                        payload.features,
                        payload.aggregation,
                        payload.status.with_default_code("ok"),
                    ),
                    Err(error) => (
                        AudioBlock::default(),
                        FeaturesBlock::default(),
                        Aggregation::default(),
                        StatusBlock::from_error(
                            "invalid_backend_response",
                            format!("backend returned invalid JSON payload: {error}"),
                        ),
                    ),
                },
                Err(error) => (
                    AudioBlock::default(),
                    FeaturesBlock::default(),
                    Aggregation::default(),
                    StatusBlock::from_error("backend_error", error.to_string()),
                ),
            };

        Ok(AnalysisRecord {
            schema: build_schema_block(),
            file: build_file_block(path, relative_path, &identity),
            audio,
            analysis: build_analysis_block(&self.config),
            features,
            aggregation,
            provenance: build_provenance_block(
                self.backend.backend_name(),
                self.backend_version.as_deref(),
            ),
            status,
        })
    }

    fn process_job(&self, job: AnalysisJob) -> Result<AnalysisRecord, PipelineError> {
        self.process_entry(&job.path, job.relative_path.as_deref(), job.identity)
    }

    fn effective_worker_count(&self) -> usize {
        let configured = self.config.performance.workers.max(1);
        std::thread::available_parallelism()
            .map(|parallelism| parallelism.get().min(configured))
            .unwrap_or(configured)
            .max(1)
    }

    fn process_jobs<I, S>(&self, jobs: I, sink: &mut S) -> Result<PipelineStats, PipelineError>
    where
        I: IntoIterator<Item = Result<AnalysisJob, PipelineError>>,
        S: RecordSink,
    {
        if self.effective_worker_count() <= 1 {
            return self.process_jobs_sequential(jobs, sink);
        }

        self.process_jobs_parallel(jobs, sink)
    }

    fn process_jobs_sequential<I, S>(
        &self,
        jobs: I,
        sink: &mut S,
    ) -> Result<PipelineStats, PipelineError>
    where
        I: IntoIterator<Item = Result<AnalysisJob, PipelineError>>,
        S: RecordSink,
    {
        let mut stats = PipelineStats::default();

        for job in jobs {
            let record = self.process_job(job?)?;
            sink.write_record(&record).map_err(PipelineError::Storage)?;
            stats.processed_files += 1;
            stats.written_records += 1;
        }

        Ok(stats)
    }

    fn process_jobs_parallel<I, S>(
        &self,
        jobs: I,
        sink: &mut S,
    ) -> Result<PipelineStats, PipelineError>
    where
        I: IntoIterator<Item = Result<AnalysisJob, PipelineError>>,
        S: RecordSink,
    {
        let worker_count = self.effective_worker_count();
        let queue_capacity = worker_count.saturating_mul(2).max(1);
        let cancel = Arc::new(AtomicBool::new(false));

        std::thread::scope(|scope| -> Result<PipelineStats, PipelineError> {
            let (job_tx, job_rx) = mpsc::sync_channel::<AnalysisJob>(queue_capacity);
            let job_rx = Arc::new(Mutex::new(job_rx));
            let (result_tx, result_rx) = mpsc::channel::<JobResult>();

            for _ in 0..worker_count {
                let job_rx = Arc::clone(&job_rx);
                let result_tx = result_tx.clone();
                let cancel = Arc::clone(&cancel);

                scope.spawn(move || {
                    loop {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }

                        let job = {
                            let receiver = job_rx.lock().expect("job receiver should lock");
                            receiver.recv()
                        };

                        let Ok(job) = job else {
                            break;
                        };
                        let index = job.index;
                        let record = self.process_job(job);

                        if result_tx.send(JobResult { index, record }).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(result_tx);

            let mut stats = PipelineStats::default();
            let mut next_to_write = 0usize;
            let mut pending = BTreeMap::new();
            let mut dispatched = 0usize;
            let mut received = 0usize;

            for job in jobs {
                let job = match job {
                    Ok(job) => job,
                    Err(error) => {
                        cancel.store(true, Ordering::Relaxed);
                        drop(job_tx);
                        return Err(error);
                    }
                };

                if job_tx.send(job).is_err() {
                    cancel.store(true, Ordering::Relaxed);
                    return Err(PipelineError::Parallel(
                        "worker channel closed before all jobs were dispatched".to_string(),
                    ));
                }
                dispatched += 1;

                if let Err(error) = drain_ready_results(
                    &result_rx,
                    &mut pending,
                    &mut next_to_write,
                    sink,
                    &mut stats,
                    &mut received,
                ) {
                    cancel.store(true, Ordering::Relaxed);
                    drop(job_tx);
                    return Err(error);
                }
            }
            drop(job_tx);

            while received < dispatched {
                let result = result_rx.recv().map_err(|_| {
                    PipelineError::Parallel(
                        "worker threads stopped before returning all records".to_string(),
                    )
                })?;
                if let Err(error) = queue_result(
                    result,
                    &mut pending,
                    &mut next_to_write,
                    sink,
                    &mut stats,
                    &mut received,
                ) {
                    cancel.store(true, Ordering::Relaxed);
                    return Err(error);
                }
            }

            Ok(stats)
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PipelineStats {
    pub processed_files: usize,
    pub written_records: usize,
}

pub trait Backend: Send + Sync {
    fn backend_name(&self) -> BackendName;
    fn backend_version(&self) -> Result<String, BackendCallError>;
    fn analyze_file(&self, path: &Path, config_json: &str) -> Result<String, BackendCallError>;
}

#[derive(Debug, Clone, Copy)]
pub struct NativeBackend {
    name: BackendName,
}

impl NativeBackend {
    pub fn new(name: BackendName) -> Self {
        Self { name }
    }
}

impl Backend for NativeBackend {
    fn backend_name(&self) -> BackendName {
        self.name
    }

    fn backend_version(&self) -> Result<String, BackendCallError> {
        audio_feature_lab_ffi::backend_version(self.name).map_err(BackendCallError::from)
    }

    fn analyze_file(&self, path: &Path, config_json: &str) -> Result<String, BackendCallError> {
        audio_feature_lab_ffi::analyze_file(self.name, path, config_json)
            .map_err(BackendCallError::from)
    }
}

#[derive(Debug)]
pub struct BackendCallError {
    message: String,
}

impl BackendCallError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BackendCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for BackendCallError {}

impl From<audio_feature_lab_ffi::BackendError> for BackendCallError {
    fn from(error: audio_feature_lab_ffi::BackendError) -> Self {
        Self::new(error.to_string())
    }
}

#[derive(Debug)]
pub enum PipelineError {
    ConfigSerialization(serde_json::Error),
    Walk(WalkError),
    Storage(StorageError),
    Parallel(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigSerialization(error) => {
                write!(f, "failed to serialize backend config JSON: {error}")
            }
            Self::Walk(error) => write!(f, "{error}"),
            Self::Storage(error) => write!(f, "{error}"),
            Self::Parallel(message) => f.write_str(message),
        }
    }
}

impl Error for PipelineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConfigSerialization(error) => Some(error),
            Self::Walk(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Parallel(_) => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct BackendPayload {
    audio: AudioBlock,
    features: FeaturesBlock,
    aggregation: Aggregation,
    status: StatusBlock,
}

#[derive(Debug)]
struct AnalysisJob {
    index: usize,
    path: PathBuf,
    relative_path: Option<PathBuf>,
    identity: FileIdentity,
}

#[derive(Debug)]
struct JobResult {
    index: usize,
    record: Result<AnalysisRecord, PipelineError>,
}

fn drain_ready_results<S>(
    result_rx: &mpsc::Receiver<JobResult>,
    pending: &mut BTreeMap<usize, Result<AnalysisRecord, PipelineError>>,
    next_to_write: &mut usize,
    sink: &mut S,
    stats: &mut PipelineStats,
    received: &mut usize,
) -> Result<(), PipelineError>
where
    S: RecordSink,
{
    while let Ok(result) = result_rx.try_recv() {
        queue_result(result, pending, next_to_write, sink, stats, received)?;
    }

    Ok(())
}

fn queue_result<S>(
    result: JobResult,
    pending: &mut BTreeMap<usize, Result<AnalysisRecord, PipelineError>>,
    next_to_write: &mut usize,
    sink: &mut S,
    stats: &mut PipelineStats,
    received: &mut usize,
) -> Result<(), PipelineError>
where
    S: RecordSink,
{
    *received += 1;
    pending.insert(result.index, result.record);
    flush_ready_results(pending, next_to_write, sink, stats)
}

fn flush_ready_results<S>(
    pending: &mut BTreeMap<usize, Result<AnalysisRecord, PipelineError>>,
    next_to_write: &mut usize,
    sink: &mut S,
    stats: &mut PipelineStats,
) -> Result<(), PipelineError>
where
    S: RecordSink,
{
    while let Some(record) = pending.remove(next_to_write) {
        let record = record?;
        sink.write_record(&record).map_err(PipelineError::Storage)?;
        stats.processed_files += 1;
        stats.written_records += 1;
        *next_to_write += 1;
    }

    Ok(())
}

impl StatusBlock {
    fn from_error(code: &str, message: String) -> Self {
        let mut fields = OrderedJsonObject::new();
        fields.insert("code".to_string(), JsonValue::String(code.to_string()));
        fields.insert("message".to_string(), JsonValue::String(message));
        Self { fields }
    }

    fn with_default_code(mut self, code: &str) -> Self {
        self.fields
            .entry("code".to_string())
            .or_insert_with(|| JsonValue::String(code.to_string()));
        self
    }
}

fn serialize_backend_config(config: &LabConfig) -> Result<String, PipelineError> {
    let value = json!({
        "profile": config.profile.as_str(),
        "backend": {
            "name": config.backend.name.as_str(),
        },
        "features": {
            "families": config.features.families.iter().map(|family| family.as_str()).collect::<Vec<_>>(),
            "enabled": config.features.enabled.iter().map(|feature| feature.as_str()).collect::<Vec<_>>(),
            "frame_level": config.features.frame_level,
        },
        "aggregation": {
            "statistics": config.aggregation.statistics.iter().map(|stat| stat.as_str()).collect::<Vec<_>>(),
        },
    });

    serde_json::to_string(&value).map_err(PipelineError::ConfigSerialization)
}

fn build_schema_block() -> FeatureSchema {
    let mut fields = OrderedJsonObject::new();
    fields.insert(
        "name".to_string(),
        JsonValue::String(SCHEMA_NAME.to_string()),
    );
    fields.insert(
        "version".to_string(),
        JsonValue::String(SCHEMA_VERSION.to_string()),
    );
    FeatureSchema { fields }
}

fn build_file_block(
    path: &Path,
    relative_path: Option<&Path>,
    identity: &FileIdentity,
) -> FileBlock {
    let mut fields = OrderedJsonObject::new();
    fields.insert(
        "path".to_string(),
        JsonValue::String(path.to_string_lossy().into_owned()),
    );
    fields.insert(
        "canonical_path".to_string(),
        path.canonicalize()
            .ok()
            .map(|canonical_path| JsonValue::String(canonical_path.to_string_lossy().into_owned()))
            .unwrap_or(JsonValue::Null),
    );
    fields.insert(
        "relative_path".to_string(),
        relative_path
            .map(|path| JsonValue::String(path.to_string_lossy().into_owned()))
            .unwrap_or(JsonValue::Null),
    );
    fields.insert(
        "size_bytes".to_string(),
        json!(identity.baseline.size_bytes),
    );
    fields.insert(
        "modified_timestamp".to_string(),
        json!(identity.baseline.modified_unix_nanos),
    );
    fields.insert(
        "identity".to_string(),
        json!({
            "modified_unix_nanos": identity.baseline.modified_unix_nanos,
            "size_bytes": identity.baseline.size_bytes,
        }),
    );
    FileBlock { fields }
}

fn build_analysis_block(config: &LabConfig) -> AnalysisBlock {
    let mut fields = OrderedJsonObject::new();
    fields.insert(
        "timestamp".to_string(),
        json!(current_unix_timestamp_millis()),
    );
    fields.insert(
        "backend".to_string(),
        JsonValue::String(config.backend.name.as_str().to_string()),
    );
    fields.insert(
        "profile".to_string(),
        JsonValue::String(config.profile.as_str().to_string()),
    );
    fields.insert(
        "frame_level".to_string(),
        JsonValue::Bool(config.features.frame_level),
    );
    fields.insert(
        "requested_families".to_string(),
        JsonValue::Array(
            config
                .features
                .families
                .iter()
                .map(|family| JsonValue::String(family.as_str().to_string()))
                .collect(),
        ),
    );
    fields.insert(
        "requested_features".to_string(),
        JsonValue::Array(
            config
                .features
                .enabled
                .iter()
                .map(|feature| JsonValue::String(feature.as_str().to_string()))
                .collect(),
        ),
    );
    fields.insert(
        "aggregation_statistics".to_string(),
        JsonValue::Array(
            config
                .aggregation
                .statistics
                .iter()
                .map(|stat| JsonValue::String(stat.as_str().to_string()))
                .collect(),
        ),
    );
    AnalysisBlock { fields }
}

fn build_provenance_block(
    backend_name: BackendName,
    backend_version: Option<&str>,
) -> ProvenanceBlock {
    let mut fields = OrderedJsonObject::new();
    fields.insert("tool".to_string(), JsonValue::String(TOOL_NAME.to_string()));
    fields.insert(
        "tool_version".to_string(),
        JsonValue::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    fields.insert(
        "schema_name".to_string(),
        JsonValue::String(SCHEMA_NAME.to_string()),
    );
    fields.insert(
        "schema_version".to_string(),
        JsonValue::String(SCHEMA_VERSION.to_string()),
    );
    fields.insert(
        "backend".to_string(),
        JsonValue::String(backend_name.as_str().to_string()),
    );
    fields.insert(
        "boundary".to_string(),
        JsonValue::String("json_string".to_string()),
    );
    fields.insert(
        "backend_version".to_string(),
        backend_version
            .map(|value| JsonValue::String(value.to_string()))
            .unwrap_or(JsonValue::Null),
    );
    ProvenanceBlock { fields }
}

fn current_unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use audio_feature_lab_config::{BackendName, LabConfig};
    use serde_json::json;

    use crate::storage::RecordSink;
    use crate::walker::WalkerConfig;

    use super::{AnalysisRecord, Backend, BackendCallError, Pipeline, PipelineStats};
    use crate::walker::Walker;

    static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn processes_single_file_into_json_ready_record() {
        let temp_dir = TestDir::new();
        let path = temp_dir.path().join("track.wav");
        write_file(&path, b"audio");

        let backend = FakeBackend::new(
            BackendName::Essentia,
            "essentia-test",
            vec![Ok(json!({
                "audio": {"sample_rate": 44100, "channels": 2},
                "features": {
                    "spectral": {"mfcc": [0.1, 0.2]},
                    "temporal": {},
                    "rhythm": {},
                    "tonal": {},
                    "dynamics": {},
                    "metadata": {"duration": 3.5},
                    "frame_level": null
                },
                "aggregation": {
                    "spectral": {"mfcc": {"mean": [0.1, 0.2]}},
                    "temporal": {},
                    "rhythm": {},
                    "tonal": {},
                    "dynamics": {},
                    "metadata": {"duration": {"mean": 3.5}}
                }
            })
            .to_string())],
        );
        let pipeline = Pipeline::new(
            LabConfig::load_default().expect("default config"),
            Walker::new(WalkerConfig::default().with_extensions(["wav"])),
            backend,
        )
        .expect("pipeline should build");

        let record = pipeline.process_file(&path).expect("file should process");

        assert_eq!(
            record.file.fields["path"],
            json!(path.to_string_lossy().to_string())
        );
        assert_eq!(record.file.fields["size_bytes"], json!(5));
        assert!(record.file.fields["canonical_path"].is_string());
        assert!(record.file.fields["modified_timestamp"].is_number());
        assert_eq!(record.schema.fields["name"], json!(super::SCHEMA_NAME));
        assert_eq!(
            record.schema.fields["version"],
            json!(super::SCHEMA_VERSION)
        );
        assert_eq!(record.analysis.fields["backend"], json!("essentia"));
        assert_eq!(record.analysis.fields["profile"], json!("default"));
        assert!(record.analysis.fields["timestamp"].is_number());
        assert_eq!(record.provenance.fields["tool"], json!(super::TOOL_NAME));
        assert_eq!(
            record.provenance.fields["tool_version"],
            json!(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(record.provenance.fields["backend"], json!("essentia"));
        assert_eq!(
            record.provenance.fields["backend_version"],
            json!("essentia-test")
        );
        assert_eq!(
            record.provenance.fields["schema_name"],
            json!(super::SCHEMA_NAME)
        );
        assert_eq!(
            record.provenance.fields["schema_version"],
            json!(super::SCHEMA_VERSION)
        );
        assert_eq!(record.status.fields["code"], json!("ok"));
        assert_eq!(record.audio.fields["sample_rate"], json!(44100));
        assert_eq!(record.aggregation.spectral.len(), 1);
    }

    #[test]
    fn backend_failures_become_error_records() {
        let temp_dir = TestDir::new();
        let path = temp_dir.path().join("track.wav");
        write_file(&path, b"audio");

        let pipeline = Pipeline::new(
            LabConfig::load_default().expect("default config"),
            Walker::default(),
            FakeBackend::new(
                BackendName::Essentia,
                "essentia-test",
                vec![Err("backend unavailable".to_string())],
            ),
        )
        .expect("pipeline should build");

        let record = pipeline
            .process_file(&path)
            .expect("record should still be built");

        assert_eq!(record.status.fields["code"], json!("backend_error"));
        assert_eq!(
            record.status.fields["message"],
            json!("backend unavailable")
        );
        assert!(record.features.spectral.is_empty());
        assert!(record.aggregation.spectral.is_empty());
    }

    #[test]
    fn records_selected_backend_in_analysis_and_provenance() {
        let temp_dir = TestDir::new();
        let path = temp_dir.path().join("track.wav");
        write_file(&path, b"audio");

        let pipeline = Pipeline::new(
            LabConfig::load_default().expect("default config"),
            Walker::default(),
            FakeBackend::new(
                BackendName::Essentia,
                "essentia-test",
                vec![Ok(success_payload().to_string())],
            ),
        )
        .expect("pipeline should build");

        let record = pipeline.process_file(&path).expect("file should process");

        assert_eq!(record.analysis.fields["backend"], json!("essentia"));
        assert_eq!(record.provenance.fields["backend"], json!("essentia"));
        assert_eq!(
            record.provenance.fields["backend_version"],
            json!("essentia-test")
        );
    }

    #[test]
    fn processes_batch_without_accumulating_in_pipeline() {
        let temp_dir = TestDir::new();
        let first = temp_dir.path().join("a.wav");
        let second = temp_dir.path().join("b.wav");
        write_file(&first, b"a");
        write_file(&second, b"b");

        let pipeline = Pipeline::new(
            LabConfig::load_default().expect("default config"),
            Walker::default(),
            FakeBackend::new(
                BackendName::Essentia,
                "essentia-test",
                vec![
                    Ok(success_payload().to_string()),
                    Ok(success_payload().to_string()),
                ],
            ),
        )
        .expect("pipeline should build");

        let mut sink = CollectingSink::default();
        let stats = pipeline
            .process_batch([first.as_path(), second.as_path()], &mut sink)
            .expect("batch should process");

        assert_eq!(
            stats,
            PipelineStats {
                processed_files: 2,
                written_records: 2,
            }
        );
        assert_eq!(sink.records.len(), 2);
    }

    #[test]
    fn process_scan_uses_walker_filtering_and_relative_paths() {
        let temp_dir = TestDir::new();
        write_file(&temp_dir.path().join("keep.wav"), b"wav");
        write_file(&temp_dir.path().join("skip.txt"), b"txt");
        write_file(&temp_dir.path().join("nested/also.FLAC"), b"flac");

        let pipeline = Pipeline::new(
            LabConfig::load_default().expect("default config"),
            Walker::new(WalkerConfig::default().with_extensions(["wav", "flac"])),
            FakeBackend::new(
                BackendName::Essentia,
                "essentia-test",
                vec![
                    Ok(success_payload().to_string()),
                    Ok(success_payload().to_string()),
                ],
            ),
        )
        .expect("pipeline should build");

        let mut sink = CollectingSink::default();
        let stats = pipeline
            .process_scan(temp_dir.path(), &mut sink)
            .expect("scan should process");

        let mut relative_paths = sink
            .records
            .iter()
            .map(|record| {
                record.file.fields["relative_path"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();
        relative_paths.sort();

        assert_eq!(stats.processed_files, 2);
        assert_eq!(relative_paths, vec!["keep.wav", "nested/also.FLAC"]);
    }

    #[test]
    fn parallel_batch_preserves_input_order() {
        let temp_dir = TestDir::new();
        let first = temp_dir.path().join("a.wav");
        let second = temp_dir.path().join("b.wav");
        let third = temp_dir.path().join("c.wav");
        write_file(&first, b"a");
        write_file(&second, b"b");
        write_file(&third, b"c");

        let mut config = LabConfig::load_default().expect("default config");
        config.performance.workers = 2;

        let pipeline = Pipeline::new(config, Walker::default(), SlowPathBackend)
            .expect("pipeline should build");

        let mut sink = CollectingSink::default();
        pipeline
            .process_batch(
                [first.as_path(), second.as_path(), third.as_path()],
                &mut sink,
            )
            .expect("batch should process");

        let written = sink
            .records
            .iter()
            .map(|record| record.file.fields["path"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            written,
            vec![
                first.to_string_lossy().to_string(),
                second.to_string_lossy().to_string(),
                third.to_string_lossy().to_string(),
            ]
        );
    }

    #[derive(Debug)]
    struct FakeBackend {
        backend_name: BackendName,
        backend_version: String,
        responses: std::sync::Mutex<VecDeque<Result<String, BackendCallError>>>,
    }

    impl FakeBackend {
        fn new(
            backend_name: BackendName,
            backend_version: &str,
            responses: Vec<Result<String, String>>,
        ) -> Self {
            Self {
                backend_name,
                backend_version: backend_version.to_string(),
                responses: std::sync::Mutex::new(
                    responses
                        .into_iter()
                        .map(|result| result.map_err(BackendCallError::new))
                        .collect(),
                ),
            }
        }
    }

    impl Backend for FakeBackend {
        fn backend_name(&self) -> BackendName {
            self.backend_name
        }

        fn backend_version(&self) -> Result<String, BackendCallError> {
            Ok(self.backend_version.clone())
        }

        fn analyze_file(
            &self,
            _path: &Path,
            _config_json: &str,
        ) -> Result<String, BackendCallError> {
            self.responses
                .lock()
                .expect("responses should lock")
                .pop_front()
                .expect("fake backend response should exist")
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct SlowPathBackend;

    impl Backend for SlowPathBackend {
        fn backend_name(&self) -> BackendName {
            BackendName::Essentia
        }

        fn backend_version(&self) -> Result<String, BackendCallError> {
            Ok("essentia-test".to_string())
        }

        fn analyze_file(
            &self,
            path: &Path,
            _config_json: &str,
        ) -> Result<String, BackendCallError> {
            let delay_ms = match path.file_name().and_then(|name| name.to_str()) {
                Some("a.wav") => 40,
                Some("b.wav") => 5,
                Some("c.wav") => 1,
                _ => 0,
            };
            thread::sleep(std::time::Duration::from_millis(delay_ms));
            Ok(success_payload().to_string())
        }
    }

    #[derive(Debug, Default)]
    struct CollectingSink {
        records: Vec<AnalysisRecord>,
    }

    impl RecordSink for CollectingSink {
        fn write_record(
            &mut self,
            record: &AnalysisRecord,
        ) -> Result<(), crate::storage::StorageError> {
            self.records.push(record.clone());
            Ok(())
        }
    }

    fn success_payload() -> serde_json::Value {
        json!({
            "audio": {"sample_rate": 44100},
            "features": {
                "spectral": {},
                "temporal": {},
                "rhythm": {},
                "tonal": {},
                "dynamics": {},
                "metadata": {},
                "frame_level": null
            },
            "aggregation": {
                "spectral": {},
                "temporal": {},
                "rhythm": {},
                "tonal": {},
                "dynamics": {},
                "metadata": {}
            }
        })
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be after epoch")
                .as_nanos();
            let id = NEXT_DIR_ID.fetch_add(1, Ordering::Relaxed);
            path.push(format!(
                "audio-feature-lab-pipeline-{}-{}-{}",
                process::id(),
                unique,
                id
            ));
            fs::create_dir_all(&path).expect("temp directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should exist");
        }

        fs::write(path, contents).expect("test file should be written");
    }
}
