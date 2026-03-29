use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use audio_feature_lab_config::LabConfig;
use audio_feature_lab_core::domain::AnalysisRecord;
use audio_feature_lab_core::pipeline::Pipeline;
use audio_feature_lab_core::storage::{JsonlWriter, RecordSink, StorageError};
use audio_feature_lab_core::walker::{Walker, WalkerConfig};

const DEFAULT_AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "flac", "mp3", "ogg", "opus", "aiff", "aif", "m4a", "aac",
];

fn main() -> ExitCode {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();

    ExitCode::from(run(env::args_os(), &mut stdout, &mut stderr))
}

fn run<I, WOut, WErr>(args: I, stdout: &mut WOut, stderr: &mut WErr) -> u8
where
    I: IntoIterator<Item = OsString>,
    WOut: Write,
    WErr: Write,
{
    let args = args.into_iter().collect::<Vec<_>>();
    let program = args
        .first()
        .and_then(|value| value.clone().into_string().ok())
        .unwrap_or_else(|| "audio-feature-lab".to_string());

    let command = match parse_command(&args[1..]) {
        Ok(command) => command,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            let _ = writeln!(stderr);
            let _ = write_help(stdout, &program);
            return 2;
        }
    };

    match command {
        Command::Help => {
            let _ = write_help(stdout, &program);
            0
        }
        Command::Version => {
            let _ = writeln!(stdout, "{} {}", program, env!("CARGO_PKG_VERSION"));
            0
        }
        Command::Analyze {
            file,
            config,
            output,
        } => handle_analyze(&file, config.as_deref(), output.as_deref(), stdout, stderr),
        Command::Batch {
            path,
            config,
            output,
        } => handle_batch(&path, config.as_deref(), output.as_deref(), stdout, stderr),
        Command::Scan { path, dry_run } => handle_scan(&path, dry_run, stdout, stderr),
        Command::BackendInfo => handle_backend_info(stdout),
        Command::Schema => handle_schema(stdout, stderr),
        Command::ValidateConfig { file } => handle_validate_config(&file, stdout, stderr),
        Command::Profiles => handle_profiles(stdout),
        Command::ExplainSchema => handle_explain_schema(stdout),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Analyze {
        file: PathBuf,
        config: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Batch {
        path: PathBuf,
        config: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Scan {
        path: PathBuf,
        dry_run: bool,
    },
    BackendInfo,
    Schema,
    ValidateConfig {
        file: PathBuf,
    },
    Profiles,
    ExplainSchema,
    Help,
    Version,
}

fn parse_command(args: &[OsString]) -> Result<Command, String> {
    let Some(command) = args.first() else {
        return Ok(Command::Help);
    };

    match command.to_str() {
        Some("-h") | Some("--help") => Ok(Command::Help),
        Some("-V") | Some("--version") => Ok(Command::Version),
        Some("analyze") => parse_analyze(&args[1..]),
        Some("batch") => parse_batch(&args[1..]),
        Some("scan") => parse_scan(&args[1..]),
        Some("backend-info") => {
            ensure_no_extra_args("backend-info", &args[1..]).map(|_| Command::BackendInfo)
        }
        Some("schema") => ensure_no_extra_args("schema", &args[1..]).map(|_| Command::Schema),
        Some("validate-config") => parse_validate_config(&args[1..]),
        Some("profiles") => ensure_no_extra_args("profiles", &args[1..]).map(|_| Command::Profiles),
        Some("explain-schema") => {
            ensure_no_extra_args("explain-schema", &args[1..]).map(|_| Command::ExplainSchema)
        }
        Some(other) => Err(format!("unknown command `{other}`")),
        None => Err("command names must be valid UTF-8".to_string()),
    }
}

fn parse_analyze(args: &[OsString]) -> Result<Command, String> {
    let mut positionals = Vec::new();
    let mut config = None;
    let mut output = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].to_str() {
            Some("--config") => {
                config = Some(read_path_flag("--config", args, &mut index)?);
            }
            Some("--output") => {
                output = Some(read_path_flag("--output", args, &mut index)?);
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unsupported flag for `analyze`: {flag}"));
            }
            _ => positionals.push(PathBuf::from(&args[index])),
        }
        index += 1;
    }

    let Some(file) = positionals.pop() else {
        return Err(
            "usage: audio-feature-lab analyze <file> [--config <file>] [--output <file>]"
                .to_string(),
        );
    };

    if !positionals.is_empty() {
        return Err(
            "usage: audio-feature-lab analyze <file> [--config <file>] [--output <file>]"
                .to_string(),
        );
    }

    Ok(Command::Analyze {
        file,
        config,
        output,
    })
}

fn parse_batch(args: &[OsString]) -> Result<Command, String> {
    let mut positionals = Vec::new();
    let mut config = None;
    let mut output = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].to_str() {
            Some("--config") => {
                config = Some(read_path_flag("--config", args, &mut index)?);
            }
            Some("--output") => {
                output = Some(read_path_flag("--output", args, &mut index)?);
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unsupported flag for `batch`: {flag}"));
            }
            _ => positionals.push(PathBuf::from(&args[index])),
        }
        index += 1;
    }

    let Some(path) = positionals.pop() else {
        return Err(
            "usage: audio-feature-lab batch <path> [--config <file>] [--output <file>]".to_string(),
        );
    };

    if !positionals.is_empty() {
        return Err(
            "usage: audio-feature-lab batch <path> [--config <file>] [--output <file>]".to_string(),
        );
    }

    Ok(Command::Batch {
        path,
        config,
        output,
    })
}

fn parse_scan(args: &[OsString]) -> Result<Command, String> {
    let mut positionals = Vec::new();
    let mut dry_run = false;

    for arg in args {
        match arg.to_str() {
            Some("--dry-run") => dry_run = true,
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unsupported flag for `scan`: {flag}"));
            }
            _ => positionals.push(PathBuf::from(arg)),
        }
    }

    let Some(path) = positionals.pop() else {
        return Err("usage: audio-feature-lab scan <path> --dry-run".to_string());
    };

    if !positionals.is_empty() {
        return Err("usage: audio-feature-lab scan <path> --dry-run".to_string());
    }

    if !dry_run {
        return Err("usage: audio-feature-lab scan <path> --dry-run".to_string());
    }

    Ok(Command::Scan { path, dry_run })
}

fn parse_validate_config(args: &[OsString]) -> Result<Command, String> {
    if args.len() != 1 {
        return Err("usage: audio-feature-lab validate-config <file>".to_string());
    }

    Ok(Command::ValidateConfig {
        file: PathBuf::from(&args[0]),
    })
}

fn ensure_no_extra_args(command: &str, args: &[OsString]) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!("usage: audio-feature-lab {command}"))
    }
}

fn read_path_flag(flag: &str, args: &[OsString], index: &mut usize) -> Result<PathBuf, String> {
    let Some(value) = args.get(*index + 1) else {
        return Err(format!("{flag} requires a path argument"));
    };
    *index += 1;
    Ok(PathBuf::from(value))
}

fn handle_analyze<WOut, WErr>(
    file: &Path,
    config_path: Option<&Path>,
    output_path: Option<&Path>,
    stdout: &mut WOut,
    stderr: &mut WErr,
) -> u8
where
    WOut: Write,
    WErr: Write,
{
    let metadata = match fs::metadata(file) {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = writeln!(stderr, "error: failed to read {}: {error}", file.display());
            return 1;
        }
    };

    if !metadata.is_file() {
        let _ = writeln!(stderr, "error: analyze expects a file path");
        return 1;
    }

    let config = match load_config(config_path) {
        Ok(config) => config,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let pipeline = match Pipeline::with_native_backend(config, default_cli_walker()) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let record = match pipeline.process_file(file) {
        Ok(record) => record,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let mut sink = match open_output_sink(output_path, stdout, stderr) {
        Ok(sink) => sink,
        Err(code) => return code,
    };

    if let Err(error) = sink.write_record(&record).and_then(|_| sink.flush()) {
        let _ = writeln!(stderr, "error: {error}");
        return 1;
    }

    if record_status_is_ok(&record) { 0 } else { 1 }
}

fn handle_batch<WOut, WErr>(
    path: &Path,
    config_path: Option<&Path>,
    output_path: Option<&Path>,
    stdout: &mut WOut,
    stderr: &mut WErr,
) -> u8
where
    WOut: Write,
    WErr: Write,
{
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = writeln!(stderr, "error: failed to read {}: {error}", path.display());
            return 1;
        }
    };

    let config = match load_config(config_path) {
        Ok(config) => config,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let pipeline = match Pipeline::with_native_backend(config, default_cli_walker()) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let sink = match open_output_sink(output_path, stdout, stderr) {
        Ok(sink) => sink,
        Err(code) => return code,
    };
    let mut sink = CountingSink::new(sink);

    let result = if metadata.is_file() {
        pipeline.process_batch([path], &mut sink)
    } else if metadata.is_dir() {
        pipeline.process_scan(path, &mut sink)
    } else {
        let _ = writeln!(stderr, "error: batch expects a file or directory path");
        return 1;
    };

    match result.and_then(|_| {
        sink.flush()
            .map_err(audio_feature_lab_core::pipeline::PipelineError::Storage)
    }) {
        Ok(_) => {
            if sink.error_records > 0 {
                1
            } else {
                0
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}

fn handle_scan<WOut, WErr>(path: &Path, dry_run: bool, stdout: &mut WOut, stderr: &mut WErr) -> u8
where
    WOut: Write,
    WErr: Write,
{
    if !dry_run {
        let _ = writeln!(
            stderr,
            "error: only `scan <path> --dry-run` is implemented at this stage"
        );
        return 2;
    }

    let walker = default_cli_walker();
    let mut matched = 0usize;

    match walker.walk(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(file) => {
                        let _ = writeln!(stdout, "{}", file.relative_path.display());
                        matched += 1;
                    }
                    Err(error) => {
                        let _ = writeln!(stderr, "error: {error}");
                        return 1;
                    }
                }
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    }

    let _ = writeln!(stderr, "matched {} candidate file(s)", matched);
    0
}

fn handle_backend_info<WOut: Write>(stdout: &mut WOut) -> u8 {
    match audio_feature_lab_ffi::backend_version() {
        Ok(version) => {
            let _ = writeln!(stdout, "backend: essentia");
            let _ = writeln!(stdout, "status: available");
            let _ = writeln!(stdout, "version: {version}");
        }
        Err(error) => {
            let _ = writeln!(stdout, "backend: essentia");
            let _ = writeln!(stdout, "status: unavailable");
            let _ = writeln!(stdout, "detail: {error}");
        }
    }
    0
}

fn handle_schema<WOut: Write, WErr: Write>(stdout: &mut WOut, stderr: &mut WErr) -> u8 {
    match serde_json::to_string_pretty(&AnalysisRecord::default()) {
        Ok(schema) => {
            let _ = writeln!(stdout, "{schema}");
            let _ = writeln!(
                stderr,
                "note: `features.frame_level` serializes as null when frame-level extraction is disabled"
            );
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "error: failed to render schema skeleton: {error}");
            1
        }
    }
}

fn handle_validate_config<WOut: Write, WErr: Write>(
    file: &Path,
    stdout: &mut WOut,
    stderr: &mut WErr,
) -> u8 {
    match LabConfig::from_path(file) {
        Ok(config) => {
            let _ = writeln!(stdout, "config is valid");
            let _ = writeln!(stdout, "profile: {}", config.profile.as_str());
            let _ = writeln!(stdout, "frame_level: {}", config.features.frame_level);
            let _ = writeln!(
                stdout,
                "feature_families: {}",
                config.features.families.len()
            );
            let _ = writeln!(
                stdout,
                "enabled_features: {}",
                config.features.enabled.len()
            );
            let _ = writeln!(
                stdout,
                "aggregation_statistics: {}",
                config.aggregation.statistics.len()
            );
            0
        }
        Err(error) => {
            let _ = writeln!(stderr, "invalid config: {error}");
            1
        }
    }
}

fn handle_profiles<WOut: Write>(stdout: &mut WOut) -> u8 {
    let _ = writeln!(
        stdout,
        "minimal  fastest; core descriptors only; no vector features"
    );
    let _ = writeln!(
        stdout,
        "default  balanced; includes mfcc; richer spectral, tonal, and rhythm"
    );
    let _ = writeln!(
        stdout,
        "research extended; band-based + gfcc + spectral_peaks; higher cost"
    );
    let _ = writeln!(stdout);
    let _ = writeln!(stdout, "frame-level extraction is disabled by default");
    let _ = writeln!(
        stdout,
        "spectral_peaks is only valid in the research profile"
    );
    0
}

fn handle_explain_schema<WOut: Write>(stdout: &mut WOut) -> u8 {
    let _ = writeln!(stdout, "top-level blocks:");
    let _ = writeln!(stdout, "- schema");
    let _ = writeln!(stdout, "- file");
    let _ = writeln!(stdout, "- audio");
    let _ = writeln!(stdout, "- analysis");
    let _ = writeln!(stdout, "- features");
    let _ = writeln!(stdout, "- aggregation");
    let _ = writeln!(stdout, "- provenance");
    let _ = writeln!(stdout, "- status");
    let _ = writeln!(stdout);
    let _ = writeln!(stdout, "rules:");
    let _ = writeln!(stdout, "- one JSON object per JSONL line");
    let _ = writeln!(stdout, "- deterministic key ordering");
    let _ = writeln!(stdout, "- no flattened aggregation keys");
    let _ = writeln!(stdout, "- vector-valued statistics remain arrays");
    0
}

fn load_config(path: Option<&Path>) -> Result<LabConfig, String> {
    match path {
        Some(path) => LabConfig::from_path(path).map_err(|error| error.to_string()),
        None => LabConfig::load_default().map_err(|error| error.to_string()),
    }
}

fn default_cli_walker() -> Walker {
    Walker::new(WalkerConfig::default().with_extensions(DEFAULT_AUDIO_EXTENSIONS))
}

fn write_help<W: Write>(stdout: &mut W, program: &str) -> io::Result<()> {
    writeln!(stdout, "Usage: {program} <command> [options]")?;
    writeln!(stdout)?;
    writeln!(stdout, "Commands:")?;
    writeln!(
        stdout,
        "  analyze <file>               Analyze a single file and emit one JSON record"
    )?;
    writeln!(
        stdout,
        "  batch <path>                 Analyze a file or directory and emit JSONL"
    )?;
    writeln!(
        stdout,
        "  scan <path> --dry-run        List candidate audio files without analysis"
    )?;
    writeln!(
        stdout,
        "  backend-info                 Report backend availability and version"
    )?;
    writeln!(
        stdout,
        "  schema                       Print the JSON output skeleton"
    )?;
    writeln!(
        stdout,
        "  validate-config <file>       Validate a TOML config file"
    )?;
    writeln!(
        stdout,
        "  profiles                     Summarize supported profiles"
    )?;
    writeln!(
        stdout,
        "  explain-schema               Explain the JSONL schema surface"
    )?;
    writeln!(stdout)?;
    writeln!(stdout, "Options:")?;
    writeln!(
        stdout,
        "  analyze|batch --config <file>  Use an explicit config instead of the default profile"
    )?;
    writeln!(
        stdout,
        "  analyze|batch --output <file>  Write JSONL output to a file instead of stdout"
    )?;
    writeln!(stdout)?;
    writeln!(stdout, "Notes:")?;
    writeln!(
        stdout,
        "  - default profile is `default` when no config is provided"
    )?;
    writeln!(stdout, "  - frame-level extraction is disabled by default")?;
    writeln!(
        stdout,
        "  - `minimal` forbids vector features; `research` is required for `spectral_peaks`"
    )?;
    Ok(())
}

fn record_status_is_ok(record: &AnalysisRecord) -> bool {
    if let Some(success) = record
        .status
        .fields
        .get("success")
        .and_then(|value| value.as_bool())
    {
        return success;
    }

    record
        .status
        .fields
        .get("code")
        .and_then(|value| value.as_str())
        == Some("ok")
}

enum OutputSink<'a, W: Write> {
    Stdout(JsonlWriter<&'a mut W>),
    File(JsonlWriter<BufWriter<File>>),
}

impl<W: Write> RecordSink for OutputSink<'_, W> {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError> {
        match self {
            Self::Stdout(writer) => writer.write_record(record),
            Self::File(writer) => writer.write_record(record),
        }
    }

    fn flush(&mut self) -> Result<(), StorageError> {
        match self {
            Self::Stdout(writer) => writer.flush(),
            Self::File(writer) => writer.flush(),
        }
    }
}

struct CountingSink<S> {
    inner: S,
    error_records: usize,
}

impl<S> CountingSink<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            error_records: 0,
        }
    }
}

impl<S: RecordSink> RecordSink for CountingSink<S> {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError> {
        if !record_status_is_ok(record) {
            self.error_records += 1;
        }
        self.inner.write_record(record)
    }

    fn flush(&mut self) -> Result<(), StorageError> {
        self.inner.flush()
    }
}

fn open_output_sink<'a, WOut, WErr>(
    output_path: Option<&Path>,
    stdout: &'a mut WOut,
    stderr: &mut WErr,
) -> Result<OutputSink<'a, WOut>, u8>
where
    WOut: Write,
    WErr: Write,
{
    match output_path {
        Some(path) => match JsonlWriter::create_file(path) {
            Ok(writer) => Ok(OutputSink::File(writer)),
            Err(error) => {
                let _ = writeln!(stderr, "error: {error}");
                Err(1)
            }
        },
        None => Ok(OutputSink::Stdout(JsonlWriter::new(stdout))),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn validate_config_smoke_test() {
        let config_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../configs/default.toml");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("validate-config"),
                config_path.into_os_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .contains("profile: default")
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn schema_smoke_test() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("schema"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("\"schema\""));
        assert!(output.contains("\"aggregation\""));
        assert!(String::from_utf8(stderr).unwrap().contains("frame_level"));
    }

    #[test]
    fn scan_dry_run_smoke_test() {
        let temp_dir = TestDir::new();
        write_file(&temp_dir.path().join("keep.wav"), b"wav");
        write_file(&temp_dir.path().join("skip.txt"), b"text");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("scan"),
                temp_dir.path().as_os_str().to_os_string(),
                OsString::from("--dry-run"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("keep.wav"));
        assert!(!output.contains("skip.txt"));
        assert!(
            String::from_utf8(stderr)
                .unwrap()
                .contains("matched 1 candidate")
        );
    }

    #[cfg(not(feature = "native-backend"))]
    #[test]
    fn analyze_smoke_test_emits_error_record_when_backend_is_unavailable() {
        let temp_dir = TestDir::new();
        let file = temp_dir.path().join("track.wav");
        write_file(&file, b"audio");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("analyze"),
                file.into_os_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 1);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("\"status\":{\"code\":\"backend_error\""));
        assert!(stderr.is_empty());
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn analyze_smoke_test_emits_real_record_with_native_backend() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/audio/short-stereo-44k.wav");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("analyze"),
                fixture.into_os_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("\"audio\":{\"channels\":2"));
        assert!(output.contains("\"status\":{\"code\":\"partial\""));
        assert!(stderr.is_empty());
    }

    #[cfg(not(feature = "native-backend"))]
    #[test]
    fn backend_info_smoke_test() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("backend-info"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("backend: essentia"));
        assert!(output.contains("status: unavailable"));
        assert!(stderr.is_empty());
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn backend_info_smoke_test_reports_available_backend() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = super::run(
            [
                OsString::from("audio-feature-lab"),
                OsString::from("backend-info"),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("backend: essentia"));
        assert!(output.contains("status: available"));
        assert!(output.contains("version: essentia"));
        assert!(stderr.is_empty());
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
                "audio-feature-lab-cli-{}-{}-{}",
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
