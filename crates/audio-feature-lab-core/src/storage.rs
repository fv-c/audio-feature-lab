use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::domain::AnalysisRecord;

pub trait RecordSink {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError>;

    fn flush(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct JsonlWriter<W> {
    writer: W,
}

impl<W> JsonlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_records<I>(&mut self, records: I) -> Result<usize, StorageError>
    where
        I: IntoIterator<Item = AnalysisRecord>,
        W: Write,
    {
        let mut written = 0;

        for record in records {
            self.write_record(&record)?;
            written += 1;
        }

        Ok(written)
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl JsonlWriter<BufWriter<File>> {
    pub fn create_file(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorageError::PathIo {
                operation: "create parent directories",
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = File::create(path).map_err(|source| StorageError::PathIo {
            operation: "create JSONL file",
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self::new(BufWriter::new(file)))
    }

    pub fn append_file(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorageError::PathIo {
                operation: "create parent directories",
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| StorageError::PathIo {
                operation: "open JSONL file for append",
                path: path.to_path_buf(),
                source,
            })?;

        Ok(Self::new(BufWriter::new(file)))
    }
}

impl<W: Write> RecordSink for JsonlWriter<W> {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError> {
        serde_json::to_writer(&mut self.writer, record).map_err(StorageError::Serialize)?;
        self.writer.write_all(b"\n").map_err(StorageError::Io)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), StorageError> {
        self.writer.flush().map_err(StorageError::Io)
    }
}

#[derive(Debug)]
pub struct JsonlReader<R> {
    reader: R,
    next_line_number: usize,
    buffer: String,
}

impl<R: BufRead> JsonlReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            next_line_number: 1,
            buffer: String::new(),
        }
    }

    pub fn validate_all(mut self) -> Result<usize, StorageError> {
        let mut count = 0;

        while let Some(record) = self.next_record()? {
            let _ = record;
            count += 1;
        }

        Ok(count)
    }

    pub fn next_record(&mut self) -> Result<Option<AnalysisRecord>, StorageError> {
        self.buffer.clear();

        let bytes = self
            .reader
            .read_line(&mut self.buffer)
            .map_err(StorageError::Io)?;
        if bytes == 0 {
            return Ok(None);
        }

        let line_number = self.next_line_number;
        self.next_line_number += 1;

        let line = self.buffer.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            return Err(StorageError::InvalidJsonlLine {
                line_number,
                source: serde_json::Error::io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "empty JSONL line",
                )),
            });
        }

        let record = serde_json::from_str::<AnalysisRecord>(line).map_err(|source| {
            StorageError::InvalidJsonlLine {
                line_number,
                source,
            }
        })?;
        Ok(Some(record))
    }
}

impl JsonlReader<BufReader<File>> {
    pub fn open_file(path: &Path) -> Result<Self, StorageError> {
        let file = File::open(path).map_err(|source| StorageError::PathIo {
            operation: "open JSONL file",
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self::new(BufReader::new(file)))
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    PathIo {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Serialize(serde_json::Error),
    InvalidJsonlLine {
        line_number: usize,
        source: serde_json::Error,
    },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to write JSONL output: {error}"),
            Self::PathIo {
                operation,
                path,
                source,
            } => write!(f, "{operation} for {}: {source}", path.display()),
            Self::Serialize(error) => write!(f, "failed to serialize JSONL record: {error}"),
            Self::InvalidJsonlLine {
                line_number,
                source,
            } => write!(f, "invalid JSONL at line {line_number}: {source}"),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::PathIo { source, .. } => Some(source),
            Self::Serialize(error) => Some(error),
            Self::InvalidJsonlLine { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::domain::AnalysisRecord;

    use super::{JsonlReader, JsonlWriter, RecordSink, StorageError};

    static NEXT_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn writes_one_json_object_per_line() {
        let mut writer = JsonlWriter::new(Vec::new());
        writer
            .write_record(&sample_record())
            .expect("record should be written");
        writer
            .write_record(&sample_record())
            .expect("record should be written");
        writer.flush().expect("writer should flush");

        let output = String::from_utf8(writer.into_inner()).expect("valid utf-8");
        let lines = output.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], serde_json::to_string(&sample_record()).unwrap());
        assert_eq!(lines[1], serde_json::to_string(&sample_record()).unwrap());
    }

    #[test]
    fn appends_without_rewriting_existing_lines() {
        let temp_dir = TestDir::new();
        let path = temp_dir.path().join("records.jsonl");

        let mut first_writer = JsonlWriter::create_file(&path).expect("file should be created");
        first_writer
            .write_record(&sample_record())
            .expect("record should be written");
        first_writer.flush().expect("flush should succeed");

        let mut second_writer = JsonlWriter::append_file(&path).expect("file should open");
        second_writer
            .write_record(&sample_record())
            .expect("record should be written");
        second_writer.flush().expect("flush should succeed");

        let contents = fs::read_to_string(&path).expect("file should be readable");
        assert_eq!(contents.lines().count(), 2);
    }

    #[test]
    fn write_records_streams_multiple_records() {
        let mut writer = JsonlWriter::new(Vec::new());
        let count = writer
            .write_records(vec![sample_record(), sample_record(), sample_record()])
            .expect("records should be written");
        writer.flush().expect("flush should succeed");

        assert_eq!(count, 3);
        let output = String::from_utf8(writer.into_inner()).expect("valid utf-8");
        assert_eq!(output.lines().count(), 3);
    }

    #[test]
    fn read_back_validates_jsonl_line_by_line() {
        let mut writer = JsonlWriter::new(Vec::new());
        writer
            .write_record(&sample_record())
            .expect("record should be written");
        writer
            .write_record(&sample_record())
            .expect("record should be written");
        let bytes = writer.into_inner();

        let reader = JsonlReader::new(std::io::Cursor::new(bytes));
        let count = reader.validate_all().expect("jsonl should validate");

        assert_eq!(count, 2);
    }

    #[test]
    fn invalid_jsonl_line_reports_line_number() {
        let reader = JsonlReader::new(std::io::Cursor::new(
            b"{\"schema\":{},\"file\":{},\"audio\":{},\"analysis\":{},\"features\":{\"spectral\":{},\"temporal\":{},\"rhythm\":{},\"tonal\":{},\"dynamics\":{},\"metadata\":{},\"frame_level\":null},\"aggregation\":{\"spectral\":{},\"temporal\":{},\"rhythm\":{},\"tonal\":{},\"dynamics\":{},\"metadata\":{}},\"provenance\":{},\"status\":{}}\nnot-json\n".to_vec(),
        ));

        let error = reader.validate_all().expect_err("validation should fail");
        match error {
            StorageError::InvalidJsonlLine { line_number, .. } => assert_eq!(line_number, 2),
            other => panic!("expected invalid line error, got {other}"),
        }
    }

    fn sample_record() -> AnalysisRecord {
        AnalysisRecord::default()
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
                "audio-feature-lab-storage-{}-{}-{}",
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
}
