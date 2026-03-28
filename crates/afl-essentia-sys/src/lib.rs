#[cfg(feature = "native-backend")]
use std::ffi::c_char;

#[cfg(feature = "native-backend")]
#[link(name = "afl_essentia_wrapper")]
unsafe extern "C" {
    pub fn afl_essentia_backend_version() -> *mut c_char;
    pub fn afl_essentia_analyze_file(
        path: *const c_char,
        config_json: *const c_char,
    ) -> *mut c_char;
    pub fn afl_essentia_free_string(value: *mut c_char);
}

#[cfg(not(feature = "native-backend"))]
pub mod disabled {
    use std::ffi::c_char;

    pub unsafe fn afl_essentia_backend_version() -> *mut c_char {
        std::ptr::null_mut()
    }

    pub unsafe fn afl_essentia_analyze_file(
        _path: *const c_char,
        _config_json: *const c_char,
    ) -> *mut c_char {
        std::ptr::null_mut()
    }

    pub unsafe fn afl_essentia_free_string(_value: *mut c_char) {}
}

pub const NATIVE_BACKEND_ENABLED: bool = cfg!(feature = "native-backend");
