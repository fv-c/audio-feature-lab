use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt;
use std::path::Path;

pub fn backend_version() -> Result<String, BackendError> {
    call_backend_string(std::ptr::null(), std::ptr::null(), |_, _| unsafe {
        raw::backend_version()
    })
}

pub fn analyze_file(path: &Path, config_json: &str) -> Result<String, BackendError> {
    let path = path
        .to_str()
        .ok_or_else(|| BackendError::InvalidPathEncoding(path.to_path_buf()))?;
    let path = CString::new(path).map_err(BackendError::PathContainsNul)?;
    let config_json = CString::new(config_json).map_err(BackendError::ConfigJsonContainsNul)?;

    call_backend_string(
        path.as_ptr(),
        config_json.as_ptr(),
        |path_ptr, config_ptr| unsafe { raw::analyze_file(path_ptr, config_ptr) },
    )
}

fn call_backend_string(
    path_ptr: *const std::ffi::c_char,
    config_ptr: *const std::ffi::c_char,
    invoke: unsafe fn(*const std::ffi::c_char, *const std::ffi::c_char) -> *mut std::ffi::c_char,
) -> Result<String, BackendError> {
    if !afl_essentia_sys::NATIVE_BACKEND_ENABLED {
        return Err(BackendError::Unavailable(
            "native Essentia backend is scaffolded but not linked; enable the `native-backend` feature and provide the wrapper library".to_string(),
        ));
    }

    let response = unsafe { invoke(path_ptr, config_ptr) };
    if response.is_null() {
        return Err(BackendError::NullResponse);
    }

    let value = unsafe { CStr::from_ptr(response) }
        .to_str()
        .map_err(BackendError::InvalidUtf8Response)?
        .to_owned();

    unsafe {
        raw::free_string(response);
    }

    Ok(value)
}

mod raw {
    use std::ffi::c_char;

    pub unsafe fn backend_version() -> *mut c_char {
        #[cfg(feature = "native-backend")]
        {
            unsafe { afl_essentia_sys::afl_essentia_backend_version() }
        }

        #[cfg(not(feature = "native-backend"))]
        {
            unsafe { afl_essentia_sys::disabled::afl_essentia_backend_version() }
        }
    }

    pub unsafe fn analyze_file(path: *const c_char, config_json: *const c_char) -> *mut c_char {
        #[cfg(feature = "native-backend")]
        {
            unsafe { afl_essentia_sys::afl_essentia_analyze_file(path, config_json) }
        }

        #[cfg(not(feature = "native-backend"))]
        {
            unsafe { afl_essentia_sys::disabled::afl_essentia_analyze_file(path, config_json) }
        }
    }

    pub unsafe fn free_string(value: *mut c_char) {
        #[cfg(feature = "native-backend")]
        {
            unsafe { afl_essentia_sys::afl_essentia_free_string(value) }
        }

        #[cfg(not(feature = "native-backend"))]
        {
            unsafe { afl_essentia_sys::disabled::afl_essentia_free_string(value) }
        }
    }
}

#[derive(Debug)]
pub enum BackendError {
    Unavailable(String),
    InvalidPathEncoding(std::path::PathBuf),
    PathContainsNul(std::ffi::NulError),
    ConfigJsonContainsNul(std::ffi::NulError),
    InvalidUtf8Response(std::str::Utf8Error),
    NullResponse,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(message) => write!(f, "{message}"),
            Self::InvalidPathEncoding(path) => {
                write!(
                    f,
                    "path is not valid UTF-8 for the native wrapper: {}",
                    path.display()
                )
            }
            Self::PathContainsNul(_) => write!(f, "path contains an interior NUL byte"),
            Self::ConfigJsonContainsNul(_) => {
                write!(f, "config_json contains an interior NUL byte")
            }
            Self::InvalidUtf8Response(_) => {
                write!(f, "native wrapper returned non-UTF-8 data")
            }
            Self::NullResponse => write!(f, "native wrapper returned a null string pointer"),
        }
    }
}

impl Error for BackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PathContainsNul(source) => Some(source),
            Self::ConfigJsonContainsNul(source) => Some(source),
            Self::InvalidUtf8Response(source) => Some(source),
            Self::Unavailable(_) | Self::InvalidPathEncoding(_) | Self::NullResponse => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{BackendError, analyze_file, backend_version};

    #[cfg(not(feature = "native-backend"))]
    #[test]
    fn reports_unavailable_backend_without_native_feature() {
        let error = backend_version().expect_err("backend should be unavailable");
        assert!(matches!(error, BackendError::Unavailable(_)));
    }

    #[test]
    fn validates_path_before_calling_native_backend() {
        let error =
            analyze_file(Path::new("bad\0path.wav"), "{}").expect_err("path should be rejected");
        assert!(matches!(error, BackendError::PathContainsNul(_)));
    }

    #[test]
    fn validates_config_json_before_calling_native_backend() {
        let error =
            analyze_file(Path::new("ok.wav"), "{\0}").expect_err("config_json should be rejected");
        assert!(matches!(error, BackendError::ConfigJsonContainsNul(_)));
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn reports_real_backend_version_with_native_feature() {
        let version = backend_version().expect("native backend should be available");
        assert!(version.contains("essentia"));
        assert!(!version.contains("unconfigured"));
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn analyzes_real_fixture_with_partial_or_better_status() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/audio/short-stereo-44k.wav");
        let config = r#"{"profile":"default","features":{"families":["spectral","rhythm","tonal","dynamics","metadata"],"enabled":["centroid","spread","rolloff","flux","flatness","entropy","hfc","mfcc","tempo","beat_period","onset_strength","hpcp","chroma","key_strength","tuning_frequency","loudness","dynamic_complexity","duration","silence_ratio","active_ratio"],"frame_level":false},"aggregation":{"statistics":["mean"]}}"#;

        let response =
            analyze_file(&fixture, config).expect("native backend should analyze fixture");
        let payload: serde_json::Value =
            serde_json::from_str(&response).expect("backend response must be valid JSON");

        assert_eq!(payload["status"]["success"], serde_json::Value::Bool(true));
        assert!(payload["aggregation"]["spectral"]["centroid"]["mean"].is_number());
        assert!(payload["aggregation"]["tonal"]["hpcp"]["mean"].is_array());
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn supports_frame_level_for_available_descriptors() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/audio/short-stereo-44k.wav");
        let config = r#"{"profile":"research","features":{"families":["spectral","tonal","dynamics"],"enabled":["centroid","mfcc","hpcp","loudness_ebu"],"frame_level":true},"aggregation":{"statistics":["mean"]}}"#;

        let response =
            analyze_file(&fixture, config).expect("native backend should analyze fixture");
        let payload: serde_json::Value =
            serde_json::from_str(&response).expect("backend response must be valid JSON");

        assert_eq!(payload["status"]["success"], serde_json::Value::Bool(true));
        assert!(payload["features"]["frame_level"]["spectral"]["centroid"].is_array());
        assert!(payload["features"]["frame_level"]["spectral"]["mfcc"].is_array());
        assert!(payload["features"]["frame_level"]["tonal"]["hpcp"].is_array());
        assert!(payload["features"]["frame_level"]["dynamics"]["loudness_ebu"].is_array());
    }
}
