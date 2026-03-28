use std::error::Error;
use std::fmt;
use std::path::Path;

pub fn backend_version() -> &'static str {
    "unavailable"
}

pub fn analyze_file(_path: &Path, _config_json: &str) -> Result<String, BackendError> {
    Err(BackendError::Unavailable)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendError {
    Unavailable,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => write!(f, "Essentia backend is not integrated yet"),
        }
    }
}

impl Error for BackendError {}
