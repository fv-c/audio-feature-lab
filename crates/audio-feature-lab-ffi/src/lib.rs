use std::error::Error;
use std::fmt;
use std::path::Path;

use audio_feature_lab_config::BackendName;

static KNOWN_BACKENDS: [BackendName; 1] = [BackendName::Essentia];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub name: BackendName,
    pub available: bool,
    pub version: Option<String>,
    pub detail: Option<String>,
}

pub fn known_backends() -> &'static [BackendName] {
    &KNOWN_BACKENDS
}

pub fn backend_status(backend: BackendName) -> BackendStatus {
    match backend_version(backend) {
        Ok(version) => BackendStatus {
            name: backend,
            available: true,
            version: Some(version),
            detail: None,
        },
        Err(error) => BackendStatus {
            name: backend,
            available: false,
            version: None,
            detail: Some(error.to_string()),
        },
    }
}

pub fn backend_version(backend: BackendName) -> Result<String, BackendError> {
    match backend {
        BackendName::Essentia => afl_essentia::backend_version().map_err(BackendError::from),
    }
}

pub fn analyze_file(
    backend: BackendName,
    path: &Path,
    config_json: &str,
) -> Result<String, BackendError> {
    match backend {
        BackendName::Essentia => {
            afl_essentia::analyze_file(path, config_json).map_err(BackendError::from)
        }
    }
}

#[derive(Debug)]
pub enum BackendError {
    Essentia(afl_essentia::BackendError),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Essentia(error) => write!(f, "{error}"),
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Essentia(error) => Some(error),
        }
    }
}

impl From<afl_essentia::BackendError> for BackendError {
    fn from(error: afl_essentia::BackendError) -> Self {
        Self::Essentia(error)
    }
}

#[cfg(test)]
mod tests {
    use audio_feature_lab_config::BackendName;

    use super::known_backends;

    #[test]
    fn reports_known_backends() {
        assert_eq!(known_backends(), &[BackendName::Essentia]);
    }
}
