//#![allow(unused)]
// Lets the `stored_shaders!` macro (which emits `::sdl3_gs::...` paths)
// resolve when expanded inside this crate itself, e.g. `STORED_SHADERS`.
extern crate self as sdl3_gs;

pub mod callbacks;
pub mod device;
pub mod event;
pub mod filesystem;
pub mod logging;
pub mod properties;
pub mod shader_assets;
pub mod tools;
pub mod window;

#[cfg(feature = "shader-compiler")]
pub mod shader_build;

pub use sdl3_sys as sys;

pub use sdl3_sys::init::*;
pub use sdl3_sys::video::*;

pub use proc_macros::*;

pub fn sdl_init(flags: SDL_InitFlags) -> bool {
    unsafe { SDL_Init(flags) }
}

pub fn set_hint(name: *const core::ffi::c_char, value: &core::ffi::CStr) -> bool {
    unsafe { sdl3_sys::hints::SDL_SetHint(name, value.as_ptr()) }
}

pub fn sdl_get_error() -> String {
    unsafe { std::ffi::CStr::from_ptr(sdl3_sys::error::SDL_GetError()) }
        .to_string_lossy()
        .into_owned()
}
