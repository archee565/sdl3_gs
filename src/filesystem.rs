use sdl3_sys as sys;
use std::ffi::CStr;
use std::path::PathBuf;

fn cstr_to_string(ptr: *const core::ffi::c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
}

/// Get the directory where the application was run from.
pub fn get_base_path() -> Option<PathBuf> {
    let ptr = unsafe { sys::filesystem::SDL_GetBasePath() };
    cstr_to_string(ptr).map(PathBuf::from)
}

/// Get the per-user, writable data directory for this application.
///
/// Wraps `SDL_GetPrefPath` and returns an owned `PathBuf`; the SDL-allocated
/// buffer is freed before returning.
pub fn get_pref_path(org: &str, app: &str) -> Option<PathBuf> {
    let org = std::ffi::CString::new(org).ok()?;
    let app = std::ffi::CString::new(app).ok()?;
    let ptr = unsafe { sys::filesystem::SDL_GetPrefPath(org.as_ptr(), app.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    let path = cstr_to_string(ptr).map(PathBuf::from);
    unsafe { sys::stdinc::SDL_free(ptr.cast()) };
    path
}
