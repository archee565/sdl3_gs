//#![allow(unused)]
pub mod device;
pub mod event;
pub mod window;
pub mod tools;
pub mod callbacks;
pub mod properties;

pub use sdl3_sys as sys;

pub use sdl3_sys::init::*;
pub use sdl3_sys::video::*;

pub use proc_macros::*;

pub fn sdl_init(flags : SDL_InitFlags) -> bool
{
    unsafe
    {
        SDL_Init(flags)
    }
}

pub fn set_hint(name: *const core::ffi::c_char, value: &core::ffi::CStr) -> bool
{
    unsafe
    {
        sdl3_sys::hints::SDL_SetHint(name, value.as_ptr())
    }
}

pub fn sdl_get_error() -> String {
    unsafe { std::ffi::CStr::from_ptr(sdl3_sys::error::SDL_GetError()) }
        .to_string_lossy()
        .into_owned()
}