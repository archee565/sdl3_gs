//! Automatic logger setup for apps built on `sdl3_gs`.
//!
//! [`init`] is called for you by [`crate::callbacks`] before the SDL main
//! callbacks start. On Android it routes to `android_logger` (logcat); on
//! desktop targets it writes to stderr and appends to `sdl3_gs.log` in the
//! current directory, so output survives console-less (windowed) Windows
//! launches. The filter comes from `RUST_LOG`, defaulting to `error`.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// Initialize the global logger once. Safe to call multiple times; the first
/// call wins.
pub fn init() {
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Error)
                .with_tag("sdl3_gs"),
        );
    }
    #[cfg(not(target_os = "android"))]
    {
        let mut filter = env_filter::Builder::new();
        if let Ok(spec) = std::env::var("RUST_LOG") {
            filter.parse(&spec);
        }
        filter.filter_level(log::LevelFilter::Error);
        let filter = filter.build();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("sdl3_gs.log")
            .ok()
            .map(Mutex::new);
        let level = filter.filter();
        let _ = log::set_boxed_logger(Box::new(Logger { filter, file }));
        log::set_max_level(level);
    }
}

#[cfg(not(target_os = "android"))]
struct Logger {
    filter: env_filter::Filter,
    file: Option<Mutex<std::fs::File>>,
}

#[cfg(not(target_os = "android"))]
impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.filter.matches(record) {
            return;
        }
        let line = format!(
            "[{} {}:{}] {}\n",
            record.level(),
            record.target(),
            record.line().unwrap_or(0),
            record.args()
        );
        let _ = write!(std::io::stderr(), "{line}");
        if let Some(file) = &self.file {
            if let Ok(mut file) = file.lock() {
                let _ = file.write_all(line.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}
