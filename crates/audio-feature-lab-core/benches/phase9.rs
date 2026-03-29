use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use audio_feature_lab_config::{BackendName, FeatureFamily, FeatureName, LabConfig, Profile};
use audio_feature_lab_core::domain::AnalysisRecord;
use audio_feature_lab_core::pipeline::{Backend, BackendCallError, Pipeline};
use audio_feature_lab_core::storage::{JsonlWriter, RecordSink, StorageError};
use audio_feature_lab_core::walker::{FileIdentity, MetadataIdentity, Walker, WalkerConfig};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serde_json::{Map, Value, json};

static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

fn bench_walker(c: &mut Criterion) {
    let mut group = c.benchmark_group("walker");

    let flat = BenchmarkCorpus::new(CorpusShape::Flat {
        audio_files: 512,
        extra_non_audio_files: 128,
    });
    let flat_walker = Walker::new(default_extensions());
    group.throughput(Throughput::Elements(flat.expected_matches() as u64));
    group.bench_with_input(
        BenchmarkId::new("stream_count", "flat_512"),
        &flat,
        |b, corpus| {
            b.iter(|| {
                let count = count_walked_files(&flat_walker, corpus.root());
                black_box(count)
            });
        },
    );

    let nested = BenchmarkCorpus::new(CorpusShape::Nested {
        levels: 4,
        dirs_per_level: 3,
        files_per_dir: 24,
        hidden_files_per_dir: 2,
    });
    let nested_walker = Walker::new(default_extensions());
    group.throughput(Throughput::Elements(nested.expected_matches() as u64));
    group.bench_with_input(
        BenchmarkId::new("stream_count", "nested_mixed"),
        &nested,
        |b, corpus| {
            b.iter(|| {
                let count = count_walked_files(&nested_walker, corpus.root());
                black_box(count)
            });
        },
    );

    group.finish();
}

fn bench_pipeline_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_single_file");
    let fixture = FixtureFile::copy_named("short-mono-8k.wav");
    let walker = Walker::new(default_extensions());

    for (label, profile) in [
        ("minimal", Profile::Minimal),
        ("default", Profile::Default),
        ("research", Profile::Research),
    ] {
        let config = LabConfig::from_profile(profile).expect("profile config should load");
        let payload = backend_payload_for_config(&config);
        let pipeline = Pipeline::new(
            config,
            walker.clone(),
            StaticBackend::new("bench-backend", &payload),
        )
        .expect("pipeline should build");

        group.bench_function(label, |b| {
            b.iter(|| {
                let record = pipeline
                    .process_file(fixture.path())
                    .expect("file should process");
                black_box(record)
            });
        });
    }

    group.finish();
}

fn bench_pipeline_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_batch");
    let corpus = BenchmarkCorpus::new(CorpusShape::Nested {
        levels: 3,
        dirs_per_level: 3,
        files_per_dir: 18,
        hidden_files_per_dir: 1,
    });
    let config = LabConfig::from_profile(Profile::Default).expect("default config should load");
    let payload = backend_payload_for_config(&config);
    let pipeline = Pipeline::new(
        config,
        Walker::new(default_extensions()),
        StaticBackend::new("bench-backend", &payload),
    )
    .expect("pipeline should build");

    group.throughput(Throughput::Elements(corpus.expected_matches() as u64));
    group.bench_function("process_scan_default_profile", |b| {
        b.iter(|| {
            let mut sink = NullSink;
            let stats = pipeline
                .process_scan(corpus.root(), &mut sink)
                .expect("scan should process");
            black_box(stats)
        });
    });
    group.finish();
}

fn bench_pipeline_batch_workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_batch_workers");
    let corpus = BenchmarkCorpus::new(CorpusShape::Nested {
        levels: 3,
        dirs_per_level: 3,
        files_per_dir: 18,
        hidden_files_per_dir: 1,
    });

    for workers in [1usize, 2usize] {
        let mut config =
            LabConfig::from_profile(Profile::Default).expect("default config should load");
        config.performance.workers = workers;
        let payload = backend_payload_for_config(&config);
        let pipeline = Pipeline::new(
            config,
            Walker::new(default_extensions()),
            StaticBackend::new("bench-backend", &payload),
        )
        .expect("pipeline should build");

        group.throughput(Throughput::Elements(corpus.expected_matches() as u64));
        group.bench_function(
            BenchmarkId::new("process_scan_default_profile", workers),
            |b| {
                b.iter(|| {
                    let mut sink = NullSink;
                    let stats = pipeline
                        .process_scan(corpus.root(), &mut sink)
                        .expect("scan should process");
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

fn bench_jsonl(c: &mut Criterion) {
    let mut group = c.benchmark_group("jsonl");
    let fixture = FixtureFile::copy_named("short-stereo-44k.wav");
    let config = LabConfig::from_profile(Profile::Default).expect("default config should load");
    let payload = backend_payload_for_config(&config);
    let pipeline = Pipeline::new(
        config,
        Walker::new(default_extensions()),
        StaticBackend::new("bench-backend", &payload),
    )
    .expect("pipeline should build");
    let record = pipeline
        .process_file(fixture.path())
        .expect("fixture should process");
    let records = vec![record; 256];

    group.throughput(Throughput::Elements(records.len() as u64));
    group.bench_function("write_256_records_to_memory", |b| {
        b.iter(|| {
            let mut writer = JsonlWriter::new(Vec::with_capacity(512 * 1024));
            for record in &records {
                writer
                    .write_record(record)
                    .expect("record should serialize into JSONL");
            }
            writer.flush().expect("writer should flush");
            black_box(writer.into_inner().len())
        });
    });

    group.finish();
}

fn bench_skip_logic(c: &mut Criterion) {
    let mut group = c.benchmark_group("skip_logic");
    let file_count = 4_096usize;
    let previous = previous_snapshot(file_count);
    let current_all_same = current_snapshot(file_count, 0);
    let current_mixed = current_snapshot(file_count, 3);

    group.throughput(Throughput::Elements(file_count as u64));
    group.bench_function("all_hits_mtime_size", |b| {
        b.iter(|| {
            let unchanged = count_unchanged_files(&current_all_same, &previous);
            black_box(unchanged)
        });
    });
    group.bench_function("mixed_hits_mtime_size", |b| {
        b.iter(|| {
            let unchanged = count_unchanged_files(&current_mixed, &previous);
            black_box(unchanged)
        });
    });
    group.finish();
}

#[cfg(feature = "native-backend")]
fn bench_native_pipeline_single_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_single_file_native");
    let fixture = fixture_path("short-stereo-44k.wav");
    let walker = Walker::new(default_extensions());

    for (label, profile) in [
        ("minimal", Profile::Minimal),
        ("default", Profile::Default),
        ("research", Profile::Research),
    ] {
        let config = LabConfig::from_profile(profile).expect("profile config should load");
        let pipeline = Pipeline::with_configured_backend(config, walker.clone())
            .expect("native pipeline should build");

        group.bench_function(label, |b| {
            b.iter(|| {
                let record = pipeline
                    .process_file(&fixture)
                    .expect("native backend should process fixture");
                black_box(record)
            });
        });
    }

    group.finish();
}

#[cfg(feature = "native-backend")]
fn bench_native_pipeline_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_batch_native");
    let corpus = NativeBenchmarkCorpus::new(12);
    let config = LabConfig::from_profile(Profile::Default).expect("default config should load");
    let pipeline = Pipeline::with_configured_backend(config, Walker::new(default_extensions()))
        .expect("native pipeline should build");

    group.throughput(Throughput::Elements(corpus.file_count() as u64));
    group.bench_function("process_scan_default_profile", |b| {
        b.iter(|| {
            let mut sink = NullSink;
            let stats = pipeline
                .process_scan(corpus.root(), &mut sink)
                .expect("native scan should process");
            black_box(stats)
        });
    });
    group.finish();
}

#[cfg(feature = "native-backend")]
fn bench_native_pipeline_batch_workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_batch_native_workers");
    let corpus = NativeBenchmarkCorpus::new(8);

    for workers in [1usize, 2usize] {
        let mut config =
            LabConfig::from_profile(Profile::Default).expect("default config should load");
        config.performance.workers = workers;
        let pipeline = Pipeline::with_configured_backend(config, Walker::new(default_extensions()))
            .expect("native pipeline should build");

        group.throughput(Throughput::Elements(corpus.file_count() as u64));
        group.bench_function(
            BenchmarkId::new("process_scan_default_profile", workers),
            |b| {
                b.iter(|| {
                    let mut sink = NullSink;
                    let stats = pipeline
                        .process_scan(corpus.root(), &mut sink)
                        .expect("native scan should process");
                    black_box(stats)
                });
            },
        );
    }

    group.finish();
}

#[cfg(feature = "native-backend")]
criterion_group!(
    benches,
    bench_walker,
    bench_pipeline_single_file,
    bench_pipeline_batch,
    bench_pipeline_batch_workers,
    bench_jsonl,
    bench_skip_logic,
    bench_native_pipeline_single_file,
    bench_native_pipeline_batch,
    bench_native_pipeline_batch_workers
);
#[cfg(not(feature = "native-backend"))]
criterion_group!(
    benches,
    bench_walker,
    bench_pipeline_single_file,
    bench_pipeline_batch,
    bench_pipeline_batch_workers,
    bench_jsonl,
    bench_skip_logic
);
criterion_main!(benches);

fn default_extensions() -> WalkerConfig {
    WalkerConfig::default().with_extensions(["wav", "flac", "mp3", "aiff", "aif"])
}

fn count_walked_files(walker: &Walker, root: &Path) -> usize {
    walker
        .walk(root)
        .expect("walker should open corpus")
        .fold(0, |count, entry| {
            let _ = entry.expect("entry should be readable");
            count + 1
        })
}

fn previous_snapshot(count: usize) -> BTreeMap<PathBuf, FileIdentity> {
    (0..count)
        .map(|index| {
            let path = PathBuf::from(format!("corpus/file-{index:05}.wav"));
            let identity = benchmark_identity(index, 0);
            (path, identity)
        })
        .collect()
}

fn current_snapshot(count: usize, change_stride: usize) -> Vec<(PathBuf, FileIdentity)> {
    (0..count)
        .map(|index| {
            let revision = if change_stride != 0 && index % change_stride == 0 {
                1
            } else {
                0
            };
            (
                PathBuf::from(format!("corpus/file-{index:05}.wav")),
                benchmark_identity(index, revision),
            )
        })
        .collect()
}

fn benchmark_identity(index: usize, revision: usize) -> FileIdentity {
    FileIdentity {
        baseline: MetadataIdentity {
            modified_unix_nanos: 1_700_000_000_000_000_000i128 + index as i128 + revision as i128,
            size_bytes: 4_096 + index as u64 + revision as u64,
        },
        content_hash: None,
    }
}

fn count_unchanged_files(
    current: &[(PathBuf, FileIdentity)],
    previous: &BTreeMap<PathBuf, FileIdentity>,
) -> usize {
    current
        .iter()
        .filter(|(path, identity)| {
            previous
                .get(path)
                .is_some_and(|prior| prior.same_baseline(identity))
        })
        .count()
}

fn backend_payload_for_config(config: &LabConfig) -> String {
    let mut features = family_maps();
    let mut aggregation = family_maps();

    for feature in &config.features.enabled {
        let family_key = feature.family().as_str();
        let feature_value = sample_value(*feature);
        features
            .get_mut(family_key)
            .expect("feature family should exist")
            .insert(feature.as_str().to_string(), feature_value.clone());

        let mut statistics = Map::new();
        for statistic in &config.aggregation.statistics {
            statistics.insert(statistic.as_str().to_string(), feature_value.clone());
        }
        aggregation
            .get_mut(family_key)
            .expect("aggregation family should exist")
            .insert(feature.as_str().to_string(), Value::Object(statistics));
    }

    let payload = json!({
        "audio": {
            "sample_rate": 44100,
            "channels": 2,
            "duration_seconds": 1.25
        },
        "features": {
            "spectral": Value::Object(features.remove("spectral").expect("spectral map")),
            "temporal": Value::Object(features.remove("temporal").expect("temporal map")),
            "rhythm": Value::Object(features.remove("rhythm").expect("rhythm map")),
            "tonal": Value::Object(features.remove("tonal").expect("tonal map")),
            "dynamics": Value::Object(features.remove("dynamics").expect("dynamics map")),
            "metadata": Value::Object(features.remove("metadata").expect("metadata map")),
            "frame_level": if config.features.frame_level {
                json!({
                    "spectral": {},
                    "temporal": {},
                    "rhythm": {},
                    "tonal": {},
                    "dynamics": {},
                    "metadata": {},
                })
            } else {
                Value::Null
            },
        },
        "aggregation": {
            "spectral": Value::Object(aggregation.remove("spectral").expect("spectral map")),
            "temporal": Value::Object(aggregation.remove("temporal").expect("temporal map")),
            "rhythm": Value::Object(aggregation.remove("rhythm").expect("rhythm map")),
            "tonal": Value::Object(aggregation.remove("tonal").expect("tonal map")),
            "dynamics": Value::Object(aggregation.remove("dynamics").expect("dynamics map")),
            "metadata": Value::Object(aggregation.remove("metadata").expect("metadata map")),
        },
        "status": {
            "code": "ok"
        }
    });

    serde_json::to_string(&payload).expect("benchmark payload should serialize")
}

fn family_maps() -> BTreeMap<&'static str, Map<String, Value>> {
    [
        FeatureFamily::Spectral,
        FeatureFamily::Temporal,
        FeatureFamily::Rhythm,
        FeatureFamily::Tonal,
        FeatureFamily::Dynamics,
        FeatureFamily::Metadata,
    ]
    .into_iter()
    .map(|family| (family.as_str(), Map::new()))
    .collect()
}

fn sample_value(feature: FeatureName) -> Value {
    if feature.is_vector() {
        Value::Array(vec![json!(0.1), json!(0.2), json!(0.3), json!(0.4)])
    } else {
        json!(0.5)
    }
}

#[derive(Debug, Clone)]
struct StaticBackend {
    backend_version: String,
    response: String,
}

impl StaticBackend {
    fn new(backend_version: &str, response: &str) -> Self {
        Self {
            backend_version: backend_version.to_string(),
            response: response.to_string(),
        }
    }
}

impl Backend for StaticBackend {
    fn backend_name(&self) -> BackendName {
        BackendName::Essentia
    }

    fn backend_version(&self) -> Result<String, BackendCallError> {
        Ok(self.backend_version.clone())
    }

    fn analyze_file(&self, _path: &Path, _config_json: &str) -> Result<String, BackendCallError> {
        Ok(self.response.clone())
    }
}

#[derive(Debug, Default)]
struct NullSink;

impl RecordSink for NullSink {
    fn write_record(&mut self, _record: &AnalysisRecord) -> Result<(), StorageError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum CorpusShape {
    Flat {
        audio_files: usize,
        extra_non_audio_files: usize,
    },
    Nested {
        levels: usize,
        dirs_per_level: usize,
        files_per_dir: usize,
        hidden_files_per_dir: usize,
    },
}

#[derive(Debug)]
struct BenchmarkCorpus {
    root: TestDir,
    expected_matches: usize,
}

impl BenchmarkCorpus {
    fn new(shape: CorpusShape) -> Self {
        let root = TestDir::new("bench-corpus");
        let audio_fixtures = [
            fixture_path("short-mono-8k.wav"),
            fixture_path("short-stereo-44k.wav"),
        ];

        let expected_matches = match shape {
            CorpusShape::Flat {
                audio_files,
                extra_non_audio_files,
            } => {
                for index in 0..audio_files {
                    let source = &audio_fixtures[index % audio_fixtures.len()];
                    let extension = if index % 2 == 0 { "wav" } else { "flac" };
                    copy_fixture(
                        source,
                        &root.path().join(format!("audio-{index:04}.{extension}")),
                    );
                }

                for index in 0..extra_non_audio_files {
                    write_text_file(&root.path().join(format!("note-{index:04}.txt")), "note");
                }

                audio_files
            }
            CorpusShape::Nested {
                levels,
                dirs_per_level,
                files_per_dir,
                hidden_files_per_dir,
            } => {
                let mut expected_matches = 0;
                let mut frontier = vec![root.path().to_path_buf()];

                for level in 0..levels {
                    let mut next_frontier = Vec::new();
                    for (parent_index, parent) in frontier.iter().enumerate() {
                        for dir_index in 0..dirs_per_level {
                            let dir = parent
                                .join(format!("level-{level}/dir-{parent_index}-{dir_index}"));
                            fs::create_dir_all(&dir).expect("benchmark directory should exist");
                            for file_index in 0..files_per_dir {
                                let source = &audio_fixtures[file_index % audio_fixtures.len()];
                                let extension = match file_index % 3 {
                                    0 => "wav",
                                    1 => "aiff",
                                    _ => "mp3",
                                };
                                copy_fixture(
                                    source,
                                    &dir.join(format!(
                                        "clip-{level}-{dir_index}-{file_index}.{extension}"
                                    )),
                                );
                                expected_matches += 1;
                            }
                            for hidden_index in 0..hidden_files_per_dir {
                                copy_fixture(
                                    &audio_fixtures[hidden_index % audio_fixtures.len()],
                                    &dir.join(format!(
                                        ".hidden-{level}-{dir_index}-{hidden_index}.wav"
                                    )),
                                );
                            }
                            write_text_file(&dir.join("ignore.md"), "skip");
                            next_frontier.push(dir);
                        }
                    }
                    frontier = next_frontier;
                }

                expected_matches
            }
        };

        Self {
            root,
            expected_matches,
        }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn expected_matches(&self) -> usize {
        self.expected_matches
    }
}

#[derive(Debug)]
struct FixtureFile {
    _inner: TestDir,
    path: PathBuf,
}

#[cfg(feature = "native-backend")]
#[derive(Debug)]
struct NativeBenchmarkCorpus {
    root: TestDir,
    file_count: usize,
}

#[cfg(feature = "native-backend")]
impl NativeBenchmarkCorpus {
    fn new(file_count: usize) -> Self {
        let root = TestDir::new("bench-native-corpus");
        let fixture = fixture_path("short-stereo-44k.wav");

        for index in 0..file_count {
            let parent = root.path().join(format!("set-{}/nested", index % 3));
            fs::create_dir_all(&parent).expect("native benchmark directory should exist");
            copy_fixture(&fixture, &parent.join(format!("audio-{index:04}.wav")));
        }

        Self { root, file_count }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn file_count(&self) -> usize {
        self.file_count
    }
}

impl FixtureFile {
    fn copy_named(name: &str) -> Self {
        let inner = TestDir::new("bench-file");
        let path = inner.path().join(name);
        copy_fixture(&fixture_path(name), &path);
        Self {
            _inner: inner,
            path,
        }
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

#[derive(Debug)]
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let id = NEXT_DIR_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "audio-feature-lab-{label}-{}-{}-{}",
            process::id(),
            unique,
            id
        ));
        fs::create_dir_all(&path).expect("temporary benchmark directory should be created");
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

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/audio")
        .join(name)
}

fn copy_fixture(source: &Path, target: &Path) {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("target directory should exist");
    }
    fs::copy(source, target).expect("fixture should copy");
}

fn write_text_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("target directory should exist");
    }
    fs::write(path, contents).expect("text file should be written");
}
