use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

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

    let pipeline = match Pipeline::with_configured_backend(config, default_cli_walker()) {
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

    let pipeline = match Pipeline::with_configured_backend(config, default_cli_walker()) {
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
    let progress =
        TerminalProgress::maybe_new(stderr, if metadata.is_file() { Some(1) } else { None });
    let mut sink = CountingSink::new(sink, progress);

    let result = if metadata.is_file() {
        pipeline.process_batch([path], &mut sink)
    } else if metadata.is_dir() {
        pipeline.process_scan(path, &mut sink)
    } else {
        let _ = writeln!(stderr, "error: batch expects a file or directory path");
        return 1;
    };

    let outcome = result.and_then(|_| {
        sink.flush()
            .map_err(audio_feature_lab_core::pipeline::PipelineError::Storage)
    });
    sink.finish_progress();
    sink.write_summary(stderr);

    match outcome {
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
    for backend in audio_feature_lab_ffi::known_backends() {
        let status = audio_feature_lab_ffi::backend_status(*backend);
        let _ = writeln!(stdout, "backend: {}", backend.as_str());
        let _ = writeln!(
            stdout,
            "status: {}",
            if status.available {
                "available"
            } else {
                "unavailable"
            }
        );
        if let Some(version) = status.version {
            let _ = writeln!(stdout, "version: {version}");
        }
        if let Some(detail) = status.detail {
            let _ = writeln!(stdout, "detail: {detail}");
        }
        let _ = writeln!(stdout);
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
            let _ = writeln!(stdout, "backend: {}", config.backend.name.as_str());
            let _ = writeln!(stdout, "frame_level: {}", config.features.frame_level);
            let _ = writeln!(stdout, "workers: {}", config.performance.workers);
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
    writeln!(
        stdout,
        "  - backend selection lives in `[backend].name`; shipped configs use `essentia`"
    )?;
    writeln!(stdout, "  - frame-level extraction is disabled by default")?;
    writeln!(
        stdout,
        "  - batch uses bounded workers from `[performance].workers`, clamped to available CPUs"
    )?;
    writeln!(
        stdout,
        "  - interactive batch progress and final batch summaries with failed-file details are rendered on stderr and do not alter JSONL output"
    )?;
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
    processed_records: usize,
    error_records: usize,
    failed_records: Vec<FailedRecord>,
    progress: Option<TerminalProgress>,
}

impl<S> CountingSink<S> {
    fn new(inner: S, progress: Option<TerminalProgress>) -> Self {
        Self {
            inner,
            processed_records: 0,
            error_records: 0,
            failed_records: Vec::new(),
            progress,
        }
    }

    fn finish_progress(&mut self) {
        if let Some(progress) = self.progress.as_mut() {
            progress.finish();
        }
    }

    fn write_summary<W: Write>(&self, stderr: &mut W) {
        let ok_records = self.processed_records.saturating_sub(self.error_records);
        let _ = writeln!(
            stderr,
            "summary: processed {} file(s), ok {}, error {}",
            self.processed_records, ok_records, self.error_records
        );

        const MAX_FAILED_RECORDS: usize = 10;
        for failed in self.failed_records.iter().take(MAX_FAILED_RECORDS) {
            match &failed.message {
                Some(message) if !message.is_empty() => {
                    let _ = writeln!(
                        stderr,
                        "failed: {} [{}] {}",
                        failed.path, failed.code, message
                    );
                }
                _ => {
                    let _ = writeln!(stderr, "failed: {} [{}]", failed.path, failed.code);
                }
            }
        }

        if self.failed_records.len() > MAX_FAILED_RECORDS {
            let remaining = self.failed_records.len() - MAX_FAILED_RECORDS;
            let _ = writeln!(stderr, "failed: ... and {remaining} more");
        }
    }
}

impl<S: RecordSink> RecordSink for CountingSink<S> {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError> {
        let ok = record_status_is_ok(record);
        self.processed_records += 1;
        if !ok {
            self.error_records += 1;
            self.failed_records.push(FailedRecord {
                path: record_display_path(record),
                code: record_status_code(record).to_string(),
                message: record_status_message(record),
            });
        }
        self.inner.write_record(record)?;
        if let Some(progress) = self.progress.as_mut() {
            progress.on_record(ok);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StorageError> {
        self.inner.flush()
    }
}

struct FailedRecord {
    path: String,
    code: String,
    message: Option<String>,
}

struct TerminalProgress {
    total: Option<usize>,
    processed: usize,
    error_records: usize,
    tick: usize,
    start: Instant,
    stderr: io::Stderr,
    finished: bool,
    last_line_len: usize,
}

impl TerminalProgress {
    fn maybe_new<W: Write>(_stderr: &mut W, total: Option<usize>) -> Option<Self> {
        let stderr = io::stderr();
        if !stderr.is_terminal() {
            return None;
        }

        let mut progress = Self {
            total,
            processed: 0,
            error_records: 0,
            tick: 0,
            start: Instant::now(),
            stderr,
            finished: false,
            last_line_len: 0,
        };
        progress.render();
        Some(progress)
    }

    fn on_record(&mut self, ok: bool) {
        self.processed += 1;
        if !ok {
            self.error_records += 1;
        }
        self.tick += 1;
        self.render();
    }

    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let elapsed = self.start.elapsed().as_secs_f32();
        let line = format_progress_line(
            self.total,
            self.processed,
            self.error_records,
            self.tick,
            Some(elapsed),
        );
        self.write_line(&line, true);
    }

    fn render(&mut self) {
        let elapsed = self.start.elapsed().as_secs_f32();
        let line = format_progress_line(
            self.total,
            self.processed,
            self.error_records,
            self.tick,
            Some(elapsed),
        );
        self.write_line(&line, false);
    }

    fn write_line(&mut self, line: &str, finished: bool) {
        let padding = self.last_line_len.saturating_sub(line.len());
        let mut stderr = self.stderr.lock();
        let _ = write!(stderr, "\r{line}{}", " ".repeat(padding));
        if finished {
            let _ = writeln!(stderr);
        }
        let _ = stderr.flush();
        self.last_line_len = line.len();
    }
}

fn format_progress_line(
    total: Option<usize>,
    processed: usize,
    error_records: usize,
    tick: usize,
    elapsed_seconds: Option<f32>,
) -> String {
    let ok_records = processed.saturating_sub(error_records);
    let rate = elapsed_seconds
        .filter(|elapsed| *elapsed > 0.0)
        .map(|elapsed| processed as f32 / elapsed);

    let mut line = match total {
        Some(total) => {
            let bar = render_determinate_bar(processed, total, 24);
            let percent = if total == 0 {
                100usize
            } else {
                (processed.min(total) * 100) / total
            };
            format!(
                "[{bar}] {processed}/{total} file(s) ({percent:>3}%) | ok {ok_records} | err {error_records}"
            )
        }
        None => {
            let spinner = spinner_frame(tick);
            format!("[{spinner}] {processed} file(s) | ok {ok_records} | err {error_records}")
        }
    };

    if let Some(elapsed_seconds) = elapsed_seconds {
        line.push_str(&format!(" | {elapsed_seconds:.1}s"));
        if let Some(rate) = rate {
            line.push_str(&format!(" | {rate:.2} file/s"));
        }
    }

    line
}

fn record_status_code(record: &AnalysisRecord) -> &str {
    record
        .status
        .fields
        .get("code")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
}

fn record_display_path(record: &AnalysisRecord) -> String {
    if let Some(path) = record
        .file
        .fields
        .get("relative_path")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
    {
        return path.to_string();
    }

    record
        .file
        .fields
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("<unknown>")
        .to_string()
}

fn record_status_message(record: &AnalysisRecord) -> Option<String> {
    if let Some(message) = record
        .status
        .fields
        .get("message")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
    {
        return Some(message.to_string());
    }

    record
        .status
        .fields
        .get("errors")
        .and_then(|value| value.as_array())
        .and_then(|errors| {
            errors.iter().find_map(|value| {
                value
                    .as_str()
                    .filter(|message| !message.is_empty())
                    .map(|message| message.to_string())
            })
        })
}

fn render_determinate_bar(processed: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return " ".repeat(width);
    }

    let filled = (processed.min(total) * width) / total;
    let mut bar = String::with_capacity(width);
    for index in 0..width {
        bar.push(if index < filled { '#' } else { '-' });
    }
    bar
}

fn spinner_frame(tick: usize) -> char {
    match tick % 4 {
        0 => '-',
        1 => '\\',
        2 => '|',
        _ => '/',
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
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("profile: default"));
        assert!(output.contains("backend: essentia"));
        assert!(output.contains("workers: 1"));
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
    fn formats_progress_line_without_total() {
        let line = super::format_progress_line(None, 3, 1, 2, Some(4.0));
        assert!(line.contains("3 file(s)"));
        assert!(line.contains("ok 2"));
        assert!(line.contains("err 1"));
        assert!(line.contains("0.75 file/s"));
    }

    #[test]
    fn formats_progress_line_with_total_and_elapsed() {
        let line = super::format_progress_line(Some(4), 2, 0, 1, Some(12.3));
        assert!(line.contains("2/4 file(s)"));
        assert!(line.contains("( 50%)"));
        assert!(line.contains("ok 2"));
        assert!(line.contains("12.3s"));
    }

    #[test]
    fn uses_relative_path_for_failed_record_summary_when_available() {
        let mut record = audio_feature_lab_core::domain::AnalysisRecord::default();
        record
            .file
            .fields
            .insert("path".to_string(), serde_json::json!("/abs/path/file.wav"));
        record.file.fields.insert(
            "relative_path".to_string(),
            serde_json::json!("nested/file.wav"),
        );
        record
            .status
            .fields
            .insert("code".to_string(), serde_json::json!("backend_error"));
        record
            .status
            .fields
            .insert("message".to_string(), serde_json::json!("decoder failed"));

        assert_eq!(super::record_display_path(&record), "nested/file.wav");
        assert_eq!(super::record_status_code(&record), "backend_error");
        assert_eq!(
            super::record_status_message(&record),
            Some("decoder failed".to_string())
        );
    }

    #[test]
    fn falls_back_to_absolute_path_for_failed_record_summary() {
        let mut record = audio_feature_lab_core::domain::AnalysisRecord::default();
        record
            .file
            .fields
            .insert("path".to_string(), serde_json::json!("/abs/path/file.wav"));

        assert_eq!(super::record_display_path(&record), "/abs/path/file.wav");
        assert_eq!(super::record_status_code(&record), "unknown");
        assert_eq!(super::record_status_message(&record), None);
    }

    #[test]
    fn falls_back_to_first_status_error_when_message_is_missing() {
        let mut record = audio_feature_lab_core::domain::AnalysisRecord::default();
        record.status.fields.insert(
            "errors".to_string(),
            serde_json::json!(["decoder failed", "secondary detail"]),
        );

        assert_eq!(
            super::record_status_message(&record),
            Some("decoder failed".to_string())
        );
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
        assert!(output.contains("backend: mpeg7"));
        assert!(output.contains("mpeg7 backend is unavailable"));
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
        assert!(output.contains("backend: mpeg7"));
        assert!(output.contains("mpeg7 backend is unavailable"));
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
