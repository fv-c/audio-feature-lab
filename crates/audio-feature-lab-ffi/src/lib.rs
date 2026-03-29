mod mpeg7;

use std::error::Error;
use std::fmt;
use std::path::Path;

use audio_feature_lab_config::BackendName;

static KNOWN_BACKENDS: [BackendName; 2] = [BackendName::Essentia, BackendName::Mpeg7];

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
        BackendName::Mpeg7 => mpeg7::backend_version().map_err(BackendError::from),
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
        BackendName::Mpeg7 => mpeg7::analyze_file(path, config_json).map_err(BackendError::from),
    }
}

#[derive(Debug)]
pub enum BackendError {
    Essentia(afl_essentia::BackendError),
    Mpeg7(mpeg7::BackendError),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Essentia(error) => write!(f, "{error}"),
            Self::Mpeg7(error) => write!(f, "{error}"),
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Essentia(error) => Some(error),
            Self::Mpeg7(error) => Some(error),
        }
    }
}

impl From<afl_essentia::BackendError> for BackendError {
    fn from(error: afl_essentia::BackendError) -> Self {
        Self::Essentia(error)
    }
}

impl From<mpeg7::BackendError> for BackendError {
    fn from(error: mpeg7::BackendError) -> Self {
        Self::Mpeg7(error)
    }
}

#[cfg(test)]
mod tests {
    use audio_feature_lab_config::BackendName;

    use super::{BackendError, backend_status, backend_version, known_backends};

    #[test]
    fn reports_known_backends() {
        assert_eq!(
            known_backends(),
            &[BackendName::Essentia, BackendName::Mpeg7]
        );
    }

    #[test]
    fn reports_mpeg7_backend_unavailable_without_runtime() {
        let status = backend_status(BackendName::Mpeg7);
        if status.available {
            assert!(status.version.is_some());
        } else {
            assert_eq!(status.name, BackendName::Mpeg7);
            assert!(status.version.is_none());
            assert!(
                status
                    .detail
                    .unwrap()
                    .contains("mpeg7 backend is unavailable")
            );
        }
    }

    #[test]
    fn backend_version_reports_mpeg7_unavailable_or_version() {
        match backend_version(BackendName::Mpeg7) {
            Ok(version) => assert!(!version.is_empty()),
            Err(error) => {
                assert!(matches!(error, BackendError::Mpeg7(_)));
                assert!(error.to_string().contains("mpeg7 backend"));
            }
        }
    }
}
