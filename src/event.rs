use sdl3_sys as sys;
use sys::events::*;

// Re-export types that users need to match on or inspect
pub use sys::events::SDL_EventType;
pub use sys::scancode::SDL_Scancode;
pub use sys::keycode::{SDL_Keycode, SDL_Keymod};
pub use sys::mouse::{SDL_MouseButtonFlags, SDL_MouseWheelDirection};
pub use sys::video::SDL_WindowID;

/// A parsed SDL event.
///
/// This enum provides safe, typed access to SDL3 events. Only the most common
/// event types are broken out; everything else is captured by `Other`.
pub enum Event {
    Quit {
        timestamp: u64,
    },

    // -- Window events --
    Window {
        timestamp: u64,
        window_id: SDL_WindowID,
        kind: WindowEventKind,
    },

    // -- Keyboard events --
    KeyDown {
        timestamp: u64,
        window_id: SDL_WindowID,
        scancode: SDL_Scancode,
        key: SDL_Keycode,
        r#mod: SDL_Keymod,
        repeat: bool,
    },
    KeyUp {
        timestamp: u64,
        window_id: SDL_WindowID,
        scancode: SDL_Scancode,
        key: SDL_Keycode,
        r#mod: SDL_Keymod,
    },

    // -- Mouse events --
    MouseMotion {
        timestamp: u64,
        window_id: SDL_WindowID,
        state: SDL_MouseButtonFlags,
        x: f32,
        y: f32,
        xrel: f32,
        yrel: f32,
    },
    MouseButtonDown {
        timestamp: u64,
        window_id: SDL_WindowID,
        button: u8,
        clicks: u8,
        x: f32,
        y: f32,
    },
    MouseButtonUp {
        timestamp: u64,
        window_id: SDL_WindowID,
        button: u8,
        clicks: u8,
        x: f32,
        y: f32,
    },
    MouseWheel {
        timestamp: u64,
        window_id: SDL_WindowID,
        x: f32,
        y: f32,
        direction: SDL_MouseWheelDirection,
        mouse_x: f32,
        mouse_y: f32,
    },

    /// Any event type not explicitly handled above.
    Other {
        event_type: SDL_EventType,
        timestamp: u64,
    },
}

/// Sub-types for window events, derived from [`SDL_EventType`].
pub enum WindowEventKind {
    Shown,
    Hidden,
    Exposed,
    Moved { x: i32, y: i32 },
    Resized { width: i32, height: i32 },
    PixelSizeChanged,
    Minimized,
    Maximized,
    Restored,
    MouseEnter,
    MouseLeave,
    FocusGained,
    FocusLost,
    CloseRequested,
    EnterFullscreen,
    LeaveFullscreen,
    Destroyed,
    Other(SDL_EventType),
}

/// Poll for a single pending event, returning `Some(Event)` if one was
/// available, or `None` if the queue is empty.
pub fn poll_event() -> Option<Event> {
    let mut raw = SDL_Event::default();
    let has_event = unsafe { SDL_PollEvent(&mut raw) };
    if !has_event {
        return None;
    }
    Some(parse_event(&raw))
}

/// Returns an iterator that drains all currently pending events.
pub fn poll_events() -> PollEventIter {
    PollEventIter
}

/// Iterator that yields events via [`SDL_PollEvent`] until the queue is empty.
pub struct PollEventIter;

impl Iterator for PollEventIter {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        poll_event()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_event(raw: &SDL_Event) -> Event {
    let event_type = raw.event_type();

    match event_type {
        SDL_EventType::QUIT => Event::Quit {
            timestamp: unsafe { raw.quit.timestamp },
        },

        // Window events
        t if is_window_event(t) => {
            let w = unsafe { raw.window };
            Event::Window {
                timestamp: w.timestamp,
                window_id: w.windowID,
                kind: parse_window_kind(t, w.data1, w.data2),
            }
        }

        // Keyboard
        SDL_EventType::KEY_DOWN => {
            let k = unsafe { raw.key };
            Event::KeyDown {
                timestamp: k.timestamp,
                window_id: k.windowID,
                scancode: k.scancode,
                key: k.key,
                r#mod: k.r#mod,
                repeat: k.repeat,
            }
        }
        SDL_EventType::KEY_UP => {
            let k = unsafe { raw.key };
            Event::KeyUp {
                timestamp: k.timestamp,
                window_id: k.windowID,
                scancode: k.scancode,
                key: k.key,
                r#mod: k.r#mod,
            }
        }

        // Mouse motion
        SDL_EventType::MOUSE_MOTION => {
            let m = unsafe { raw.motion };
            Event::MouseMotion {
                timestamp: m.timestamp,
                window_id: m.windowID,
                state: m.state,
                x: m.x,
                y: m.y,
                xrel: m.xrel,
                yrel: m.yrel,
            }
        }

        // Mouse buttons
        SDL_EventType::MOUSE_BUTTON_DOWN => {
            let b = unsafe { raw.button };
            Event::MouseButtonDown {
                timestamp: b.timestamp,
                window_id: b.windowID,
                button: b.button,
                clicks: b.clicks,
                x: b.x,
                y: b.y,
            }
        }
        SDL_EventType::MOUSE_BUTTON_UP => {
            let b = unsafe { raw.button };
            Event::MouseButtonUp {
                timestamp: b.timestamp,
                window_id: b.windowID,
                button: b.button,
                clicks: b.clicks,
                x: b.x,
                y: b.y,
            }
        }

        // Mouse wheel
        SDL_EventType::MOUSE_WHEEL => {
            let w = unsafe { raw.wheel };
            Event::MouseWheel {
                timestamp: w.timestamp,
                window_id: w.windowID,
                x: w.x,
                y: w.y,
                direction: w.direction,
                mouse_x: w.mouse_x,
                mouse_y: w.mouse_y,
            }
        }

        // Fallback
        _ => Event::Other {
            event_type,
            timestamp: unsafe { raw.common.timestamp },
        },
    }
}

fn is_window_event(t: SDL_EventType) -> bool {
    t.0 >= SDL_EventType::WINDOW_FIRST.0 && t.0 <= SDL_EventType::WINDOW_LAST.0
}

fn parse_window_kind(t: SDL_EventType, data1: i32, data2: i32) -> WindowEventKind {
    match t {
        SDL_EventType::WINDOW_SHOWN => WindowEventKind::Shown,
        SDL_EventType::WINDOW_HIDDEN => WindowEventKind::Hidden,
        SDL_EventType::WINDOW_EXPOSED => WindowEventKind::Exposed,
        SDL_EventType::WINDOW_MOVED => WindowEventKind::Moved { x: data1, y: data2 },
        SDL_EventType::WINDOW_RESIZED => WindowEventKind::Resized {
            width: data1,
            height: data2,
        },
        SDL_EventType::WINDOW_PIXEL_SIZE_CHANGED => WindowEventKind::PixelSizeChanged,
        SDL_EventType::WINDOW_MINIMIZED => WindowEventKind::Minimized,
        SDL_EventType::WINDOW_MAXIMIZED => WindowEventKind::Maximized,
        SDL_EventType::WINDOW_RESTORED => WindowEventKind::Restored,
        SDL_EventType::WINDOW_MOUSE_ENTER => WindowEventKind::MouseEnter,
        SDL_EventType::WINDOW_MOUSE_LEAVE => WindowEventKind::MouseLeave,
        SDL_EventType::WINDOW_FOCUS_GAINED => WindowEventKind::FocusGained,
        SDL_EventType::WINDOW_FOCUS_LOST => WindowEventKind::FocusLost,
        SDL_EventType::WINDOW_CLOSE_REQUESTED => WindowEventKind::CloseRequested,
        SDL_EventType::WINDOW_ENTER_FULLSCREEN => WindowEventKind::EnterFullscreen,
        SDL_EventType::WINDOW_LEAVE_FULLSCREEN => WindowEventKind::LeaveFullscreen,
        SDL_EventType::WINDOW_DESTROYED => WindowEventKind::Destroyed,
        other => WindowEventKind::Other(other),
    }
}
