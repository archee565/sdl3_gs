use sdl3_sys as sys;

pub use sys::properties::SDL_PropertiesID;


/// Get a string property from properties by key
pub fn get_string_property(props: SDL_PropertiesID, key: &str) -> Option<String> {
    let key_cstr = std::ffi::CString::new(key).ok()?;
    let ptr = unsafe {
        sys::properties::SDL_GetStringProperty(props, key_cstr.as_ptr(), std::ptr::null())
    };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned())
    }
}
