//! Embedded shader assets for crates whose build script runs sdl3_gs's
//! offline shader pipeline (`sdl3_gs::shader_build::compile_shaders`).
//!
//! The [`stored_shaders!`] macro (a proc macro re-exported from
//! [`crate::proc_macros`]) embeds the pipeline's output directories at
//! compile time and produces a `LazyLock<StoredShaders>` that initializes on
//! first use:
//!
//! ```ignore
//! // same directory the build script passed to compile_shaders:
//! static STORED: LazyLock<StoredShaders> = stored_shaders!("target/shaders");
//!
//! STORED.json                 // reflection JSONs (<name>.json)
//! STORED.shaders              // Vec of (dir, format), one per usable backend
//! ```
//!
//! Platform gating is baked in at expansion, so the binary only carries what
//! the target platform can consume:
//!
//! | target  | embedded dirs             | format              |
//! |---------|---------------------------|---------------------|
//! | windows | `obj_dxil/`, `obj_spirv/` | DXIL, then SPIR-V   |
//! | apple   | `obj_msl/`                | Metal (MSL)         |
//! | others  | `obj_spirv/`              | Vulkan (SPIR-V)     |
//!
//! This crate additionally provides a shared [`STORED_SHADERS`] static over
//! this crate's own pipeline output (`target/shaders`).

use crate::device::SDL_GPUShaderFormat;

/// Lazily-initialized embedded shader assets built from this crate's own
/// pipeline output: reflection JSON plus one entry per bytecode directory
/// usable on the target platform, in preference order (e.g. DXIL before
/// SPIR-V on Windows).
pub static STORED_SHADERS: std::sync::LazyLock<StoredShaders> =
    crate::stored_shaders!("target/shaders");

/// One platform-supported bytecode directory and its shader format.
pub type StoredBackend = (&'static EmbeddedDir, SDL_GPUShaderFormat);

/// Lazily-initialized embedded shader assets: reflection JSON plus one entry
/// per bytecode directory usable on this platform.
///
/// Produced by [`stored_shaders!`]; field access dereferences through
/// `LazyLock`.
#[derive(Clone)]
pub struct StoredShaders {
    /// `<root>/obj_json`, containing a `<shader name>.json` reflection file
    /// per shader (consumed via `ShaderCreateInfo::json` /
    /// `ComputePipelineCreateInfo::json`).
    pub json: &'static EmbeddedDir,
    /// Bytecode directories usable here, in preference order (e.g. DXIL
    /// before SPIR-V on Windows).
    pub shaders: Vec<StoredBackend>,
}

impl StoredShaders {
    /// Bitflags of every shader format embedded in [`StoredShaders::shaders`].
    ///
    /// Pass this to `Device::new` so backend auto-detection only considers
    /// formats that are actually available at runtime (e.g. D3D12 is only
    /// selected when this includes DXIL).
    pub fn shader_formats(&self) -> SDL_GPUShaderFormat {
        let mut formats = SDL_GPUShaderFormat::INVALID;
        for &(_, format) in &self.shaders {
            formats |= format;
        }
        formats
    }
}

/// A compile-time-embedded directory of files.
#[derive(Clone, Copy)]
pub struct EmbeddedDir {
    /// Directory name within the embedded root (`"obj_json"`, `"obj_dxil"`, ...).
    pub name: &'static str,
    /// Every file below this directory; `path`s are relative to it and use
    /// `/` separators regardless of host OS.
    pub files: &'static [EmbeddedFile],
}

/// A compile-time-embedded file.
#[derive(Clone, Copy)]
pub struct EmbeddedFile {
    /// Path relative to the containing directory (`"fill.comp.spv"`).
    pub path: &'static str,
    pub contents: &'static [u8],
}

impl EmbeddedDir {
    /// Look up a file by its path relative to this directory.
    pub fn get_file(&self, path: &str) -> Option<&EmbeddedFile> {
        self.files.iter().find(|f| f.path == path)
    }
}

impl EmbeddedFile {
    pub fn path(&self) -> &'static str {
        self.path
    }

    pub fn contents(&self) -> &'static [u8] {
        self.contents
    }
}
