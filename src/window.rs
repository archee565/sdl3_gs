use sdl3_sys as sys;
use sys::*;

pub struct Window
{
    inner : *mut sys::video::SDL_Window,
}

impl  Window {
    pub fn create(
            title: &str,
            res: (u32, u32),
            flags: sys::video::SDL_WindowFlags,
        ) -> Result<Self, String> {
        // Make sure SDL has been initialized before calling this
        // (usually done with SDL_Init or SDL_InitSubSystem)

        // Convert Rust &str → C string (null-terminated)
        let title_c = std::ffi::CString::new(title)
            .map_err(|e| format!("Invalid title string: {}", e))?;

        unsafe {
            let window_ptr = sys::video::SDL_CreateWindow(
                title_c.as_ptr(),                    // title
                res.0 as i32,                        // width
                res.1 as i32,                        // height
                flags,                        // flags (SDL_WindowFlags is usually u32)
            );

            if window_ptr.is_null() {
                // Get the SDL error message
                let error_msg = {
                    let err_ptr = sys::everything::SDL_GetError();
                    if err_ptr.is_null() {
                        "Unknown SDL error".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(err_ptr)
                            .to_string_lossy()
                            .into_owned()
                    }
                };

                return Err(format!("SDL_CreateWindow failed: {}", error_msg));
            }

            Ok(Window { inner: window_ptr })
        }
    }
    
    pub(crate) fn raw(&self) -> *mut video::SDL_Window {
        self.inner
    }
}

// Very important: we need to clean up the window when we're done
impl Drop for Window {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                sys::video::SDL_DestroyWindow(self.inner);
            }
            self.inner = std::ptr::null_mut();
        }
    }
}