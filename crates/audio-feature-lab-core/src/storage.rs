use std::error::Error;
use std::fmt;
use std::io::{self, Write};

use crate::domain::AnalysisRecord;

pub trait RecordSink {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError>;
}

#[derive(Debug)]
pub struct JsonlWriter<W> {
    writer: W,
}

impl<W> JsonlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W: Write> RecordSink for JsonlWriter<W> {
    fn write_record(&mut self, record: &AnalysisRecord) -> Result<(), StorageError> {
        serde_json::to_writer(&mut self.writer, record).map_err(StorageError::Serialize)?;
        self.writer.write_all(b"\n").map_err(StorageError::Io)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    Serialize(serde_json::Error),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to write JSONL output: {error}"),
            Self::Serialize(error) => write!(f, "failed to serialize JSONL record: {error}"),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Serialize(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::AnalysisRecord;

    use super::{JsonlWriter, RecordSink};

    #[test]
    fn writes_one_json_object_per_line() {
        let mut writer = JsonlWriter::new(Vec::new());
        writer
            .write_record(&AnalysisRecord::default())
            .expect("record should be written");
        writer
            .write_record(&AnalysisRecord::default())
            .expect("record should be written");

        let output = String::from_utf8(writer.into_inner()).expect("valid utf-8");
        let lines = output.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.starts_with('{')));
    }
}
