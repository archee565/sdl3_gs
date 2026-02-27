#![allow(unused)]
pub mod device;
pub mod event;
pub mod slot_map;
pub mod window;

pub use sdl3_sys as sys;
use sys::{*,everything::*};

pub use sdl3_sys::init::*;
pub use sdl3_sys::video::*;

pub fn sdl_init(flags : SDL_InitFlags)
{
    unsafe
    {        
        SDL_Init(flags);
    }
}