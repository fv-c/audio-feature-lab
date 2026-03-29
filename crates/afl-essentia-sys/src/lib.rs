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

    /// Returns a null pointer when the native backend is not enabled.
    ///
    /// # Safety
    ///
    /// This mirrors the native ABI. Callers must treat the returned pointer as
    /// foreign memory and must not dereference or free it unless the active
    /// backend contract explicitly allows that.
    pub unsafe fn afl_essentia_backend_version() -> *mut c_char {
        std::ptr::null_mut()
    }

    /// Returns a null pointer when the native backend is not enabled.
    ///
    /// # Safety
    ///
    /// `path` and `config_json` must follow the same validity rules as the real
    /// native ABI. Callers must not dereference or free the returned pointer
    /// unless the backend contract explicitly allows that.
    pub unsafe fn afl_essentia_analyze_file(
        _path: *const c_char,
        _config_json: *const c_char,
    ) -> *mut c_char {
        std::ptr::null_mut()
    }

    /// No-op free function used when the native backend is not enabled.
    ///
    /// # Safety
    ///
    /// The pointer must come from the matching backend allocation contract. In
    /// the disabled path this function does nothing, but callers must still
    /// uphold the ABI-level ownership expectations.
    pub unsafe fn afl_essentia_free_string(_value: *mut c_char) {}
}

pub const NATIVE_BACKEND_ENABLED: bool = cfg!(feature = "native-backend");
