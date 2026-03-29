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
        BackendName::Mpeg7 => Err(BackendError::Unavailable {
            backend,
            message:
                "MPEG-7 backend selection is wired, but no native MPEG-7 implementation is linked yet"
                    .to_string(),
        }),
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
        BackendName::Mpeg7 => Err(BackendError::Unavailable {
            backend,
            message:
                "MPEG-7 backend selection is wired, but no native MPEG-7 implementation is linked yet"
                    .to_string(),
        }),
    }
}

#[derive(Debug)]
pub enum BackendError {
    Essentia(afl_essentia::BackendError),
    Unavailable {
        backend: BackendName,
        message: String,
    },
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Essentia(error) => write!(f, "{error}"),
            Self::Unavailable { backend, message } => {
                write!(f, "{} backend is unavailable: {message}", backend.as_str())
            }
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Essentia(error) => Some(error),
            Self::Unavailable { .. } => None,
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

    use super::{BackendError, backend_status, backend_version, known_backends};

    #[test]
    fn reports_known_backends() {
        assert_eq!(
            known_backends(),
            &[BackendName::Essentia, BackendName::Mpeg7]
        );
    }

    #[test]
    fn reports_unavailable_mpeg7_backend() {
        let error = backend_version(BackendName::Mpeg7).expect_err("mpeg7 should be unavailable");
        assert!(matches!(error, BackendError::Unavailable { .. }));
        assert!(error.to_string().contains("mpeg7 backend is unavailable"));
    }

    #[test]
    fn backend_status_reflects_unavailable_mpeg7() {
        let status = backend_status(BackendName::Mpeg7);
        assert_eq!(status.name, BackendName::Mpeg7);
        assert!(!status.available);
        assert!(status.version.is_none());
        assert!(
            status
                .detail
                .unwrap()
                .contains("MPEG-7 backend selection is wired")
        );
    }
}
