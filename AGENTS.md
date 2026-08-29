# AGENTS.md

Guidance for coding agents (and humans) working on this repository.

## What this is

`sdl3_gs` is a safe Rust wrapper around SDL3's GPU API (`SDL_GPU*`), built on
[`sdl3-sys`](https://crates.io/crates/sdl3-sys) 0.6 for raw FFI bindings. The
root of this repository is both a Cargo **workspace** and the main **package**.

| Path | Crate | Purpose |
|---|---|---|
| `/` | `sdl3_gs` | The wrapper itself: device, resources, window, events, callbacks, shader pipeline |
| `/proc_macros` | `proc_macros` | Proc macros re-exported by `sdl3_gs`: `stored_shaders!` and the `SDLVertexDesc` derive |
| `/rust_shadercross` | `shadercross` | Rust port of SDL_shadercross: SPIR-V/HLSL cross-compiler + reflection (lib + CLI) |
| `/examples/demo` | `demo` (example target of `sdl3_gs`) | End-to-end demo: textured MSAA rendering + compute dispatch |

## Build & verify

```sh
cargo check --workspace      # type-check everything
cargo build                  # build the library + build scripts
cargo build --example demo   # build the demo
cargo run --example demo     # run it (needs SDL3 + a GPU/Vulkan driver)
cargo test -p shadercross    # rust_shadercross API tests (see requirements below)
```

Toolchain: Rust 2024 edition (root crate), so a recent stable Rust is required.

External system dependencies:

- **SDL3** — via `sdl3-sys`. The features `build-from-source`,
  `build-from-source-static`, `link-static`, `sdl-lean-and-mean`, and
  `link-framework` pass straight through to `sdl3-sys`.
- **glslc** (Vulkan SDK) or **glslcc** — used by the shader pipeline build
  script. `glslc` is tried first, `glslcc` as fallback.
- **shadercross requirements** (only for `rust_shadercross` and the
  `shader-compiler` feature): `dxc` with SPIR-V codegen (Vulkan SDK; override
  with `SHADERCROSS_DXC`) and the shared library `libspirv-cross-c-shared`
  (located by its `build.rs` via `SHADERCROSS_SPIRV_CROSS_DIR`, `VULKAN_SDK`,
  pkg-config, or default linker paths; an rpath is emitted so it is found at
  runtime). `shadercross` tests additionally need a SPIR-V-capable `dxc`.

Notes on the tree:

- `src/main.rs` is a vestigial "Hello, world!" binary target; the crate is a
  library. Don't build features on it.
- `examples/demo/src/*.spv` are **checked-in compiled artifacts** (`include_bytes!`
  by the demo) so the demo builds without any shader toolchain.
- `Cargo.lock` and `*.mm_profdata` (coverage profile data) are gitignored.

## Main crate (`sdl3_gs`) architecture

### Modules (`src/`)

| Module | Contents |
|---|---|
| `device.rs` | The core (~2700 lines). `Device` and every GPU resource type (`Texture`, `Shader`, `GraphicsPipeline`, `ComputePipeline`, `GPUBuffer`, `GPUTransferBuffer`, `Sampler`, `Fence`), `CommandBuffer` with `RenderPass`/`CopyPass`/`ComputePass`, upload/download helpers, blit. Also re-exports most `sdl3_sys::gpu` types used in public signatures. |
| `window.rs` | `Window` (RAII, `Drop` → `SDL_DestroyWindow`), `DisplayMode` |
| `event.rs` | Typed `Event` enum (`Quit`, `KeyDown/Up`, `Mouse*`, `Window` with `WindowEventKind`), `poll_event()` / `poll_events()` iterator; unhandled types collapse into `Event::Other` |
| `callbacks.rs` | SDL3 main-callbacks pattern: `App` trait (`init`/`iterate`/`event`/`quit`), `run::<T>()`, and the `sdl3_main!` macro exporting the `SDL_main` symbol (needed for Android/iOS; this module deliberately replaces `sdl3-sys`'s `sdl3-main`, which didn't work there) |
| `properties.rs` | `Properties` wrapper around `SDL_PropertiesID`. Owned (`Properties::new`, destroyed on drop) vs borrowed (`from_raw`, not destroyed) semantics matter — keep them straight |
| `shader_assets.rs` | `StoredShaders` / `EmbeddedDir` / `EmbeddedFile`: the types produced by the `stored_shaders!` macro (see shader pipeline below) |
| `shader_build.rs` | The offline shader pipeline, feature-gated behind `shader-compiler` (see below) |
| `tools.rs` | Older/simpler build-time helper `prepare_shaders` that shells out to the `glslc`/`glslcc`/`shadercross` CLIs. `shader_build.rs` is the current pipeline; treat `tools.rs` as legacy unless asked otherwise |
| `filesystem.rs` | `get_base_path`, `get_pref_path` |
| `lib.rs` | Module list, re-exports (`pub use sdl3_sys as sys`, init/video symbols, `proc_macros::*`), `sdl_init`, `sdl_get_error`. Note `extern crate self as sdl3_gs;` so macro-generated `::sdl3_gs::` paths resolve inside this crate too |

### Resource management model (the key invariant)

Every GPU resource follows the same pattern — keep new resources consistent
with it:

- The public type is a cheap-clone handle: `pub struct Texture { inner: Rc<TextureData> }`.
- The data struct holds the raw SDL pointer plus a **`Weak<DeviceInner>`
  back-reference** to the device (avoids Rc cycles).
- `Drop for *Data` calls the matching `SDL_ReleaseGPU*`/`SDL_Destroy*` **only
  if the `Weak` upgrades**, i.e. only while the device is alive. If the device
  was destroyed first, the SDL resource leaks (optionally logged under the
  `verbose` feature). This is accepted behavior; don't "fix" it by strong
  references or globals.
- Everything is `Rc`-based, so the whole API is **`!Send`/`!Sync`**: the crate
  targets SDL's single-threaded main-callback model.
- Nullable handles exist for every resource (`Texture::none()`,
  `GPUBuffer::none()`, …) with `is_valid()`; SDL structs that need an
  optional texture use these. Swapchain textures acquired from a command
  buffer are marked consumed (`TextureKind::None` in their `Cell`) once used
  or when the command buffer drops — `to_raw()` asserts on reuse of a
  consumed swapchain texture.

### Device and command buffers

- `Device::new(shader_formats, Option<Window>, Option<SDL_PropertiesID>)`
  creates the device, claims the window if given, and stores it in an `Rc`.
  The claimed `Window` is owned by the device afterwards (`get_window()`).
- `Device::create_buffer` zero-fills on creation (via an upload); textures,
  shaders, pipelines and samplers map 1:1 to the SDL create calls and wrap
  their `*CreateInfo` structs (the crate defines its own safe create-info
  types where SDL's would need raw pointers, e.g. `ShaderCreateInfo`,
  `GraphicsPipelineCreateInfo`, `ColorTargetInfo`, and uses `sdl3-sys`
  structs directly otherwise, e.g. `SDL_GPUTextureCreateInfo`).
- Typical frame: `device.acquire_command_buffer()` →
  `cmd.acquire_swapchain_texture()` (returns `Ok(None)` when the swapchain is
  unavailable) → `begin_render_pass`/`begin_compute_pass`/`begin_copy_pass`
  (passes borrow the command buffer and end on drop) → `cmd.submit()` or
  `cmd.submit_and_acquire_fence()`.
- `Drop for CommandBuffer` **cancels** an unsubmitted command buffer; passes
  track state in `Cell`/`RefCell` fields on the command buffer. Uploads
  without an explicit copy pass use an internally cached transfer buffer and
  a self-submitted command buffer (`Device::submit_upload`).
- Downloads (`GPUBuffer::download*`) create a transfer buffer, copy, submit,
  and wait on a fence before reading.

### Error handling convention

Fallible public functions return `Result<T, String>`; the error string is
`"<SDL call name>: <SDL_GetError()>"` (see `sdl_fail`). Programming errors
API-misuse (e.g. using a consumed swapchain texture, invalid sample count,
anisotropy with non-LINEAR filters) are `assert!`s or explicit `Err`s. Keep
this convention; do not introduce a new error type in `sdl3_gs` (`shadercross`
uses `thiserror`, which is fine there).

## The shader pipeline (end-to-end)

This is the crate's distinguishing system, spanning all three crates. The
flow for a *consumer* crate (including this one via `build.rs`):

1. **Author** GLSL under `src/shaders`. A single file may contain multiple
   entry points named `void vert_NN(` / `void frag_NN(` / `void comp_NN(`
   (exactly two digits). The two-digit suffix becomes the variant name.
2. **Build script** calls
   `sdl3_gs::shader_build::compile_shaders(&manifest/src/shaders, &manifest/target/shaders, enable_dxil, enable_msl)`
   (root `build.rs` does exactly this by `#[path]`-including
   `src/shader_build.rs`; the build script needs no feature flag).
   The pipeline: strip comments → resolve `#include "..."` (recursive,
   relative to the shader dir) → split into one file per entry point (chosen
   entry renamed to `main`, others blanked line-for-line, `#define VERT_00`
   style inserted after the leading directives) → compile with `glslc`
   (`--target-env=vulkan1.0`, `glslcc` fallback) → convert with `shadercross`
   to DXIL, MSL, and reflection JSON. Work is parallelized with `rayon` and
   incremental via mtimes; stale outputs are pruned;
   `cargo:rerun-if-changed` is emitted for sources and includes.
   Output layout under `<root>/target/shaders/`: `preprocessed/`,
   `obj_spirv/*.spv`, `obj_dxil/*.dxil`, `obj_msl/*.metal`, `obj_json/*.json`.
   The `enable_dxil`/`enable_msl` bool params toggle the optional backends.
3. **Embed** with the proc macro:
   `static STORED: LazyLock<StoredShaders> = sdl3_gs::stored_shaders!("target/shaders");`
   (path resolved against the consumer's manifest dir; must exist at macro
   expansion). It embeds `obj_json` always, plus per-platform bytecode dirs:
   Windows → `obj_dxil` + `obj_spirv`, Apple → `obj_msl`, everything else →
   `obj_spirv`. Set `SDL3_GS_DUMP_EXPANSION` to print the expansion when
   debugging the macro.
4. **Load** at runtime: `device.load_stored_shader(&STORED, "textured.vert", SDL_GPUShaderStage::VERTEX)`
   or `device.load_stored_compute_pipeline(&STORED, "fill_array.comp")`.
   Resolution: walk `StoredShaders::shaders` in preference order, take the
   first format the device supports (`Device::get_shader_formats()`), fetch
   `<name>.spv|.dxil|.metal` and `<name>.json`. Entrypoint is `main`, except
   `main0` for MSL (what spirv-cross emits).
   `StoredShaders::shader_formats()` gives the flag to pass to `Device::new`
   so backend auto-detection only considers formats that were embedded.

The reflection JSON is the contract between crates: `shadercross::json`
replicates the C SDL_shadercross CLI schema byte-for-byte, and
`Device::create_shader` / `create_compute_pipeline` extract resource counts /
threadcounts from it with a hand-rolled scanner (`parse_json_u32` looks up
`"<key>":` and parses digits — no serde anywhere in `sdl3_gs`). If you touch
`rust_shadercross/src/json.rs` or the schema, check both sides.

The `shader-compiler` feature only controls whether `sdl3_gs::shader_build`
is part of the *library* API (so consumers' build scripts can call it); the
root build script always compiles it regardless. Note that `shader_build.rs`
is compiled twice with different crate names (as build script and as library
module) — keep it free of crate-specific assumptions.

## `proc_macros` crate

- `stored_shaders!("target/shaders")` — described above; emits a
  `LazyLock<StoredShaders>` with `#[cfg]`-gated platform blocks and generates
  `::sdl3_gs::shader_assets::*` paths.
- `#[derive(SDLVertexDesc)]` — generates `vertex_desc() ->
  (Vec<SDL_GPUVertexAttribute>, Vec<SDL_GPUVertexBufferDescription>)` from a
  `#[repr(C)]` struct with named fields. Field types map to vertex formats
  (`[f32; 2]` → `FLOAT2`, `f32` → `FLOAT`, …; type names `Vec2/Vec3/Vec4/CVec*`
  accepted). Per-field attributes: `#[sdl_vertex_desc(skip)]` and
  `#[sdl_vertex_desc(format = SOME_FORMAT)]`. The derive panics on missing
  `#[repr(C)]` / unsupported types — that's intended (compile-time error).

## `rust_shadercross` (`shadercross`) crate

A Rust port of libsdl-org/SDL_shadercross (Zlib licensed, keep the header
attribution style). Lib in `src/lib.rs`, CLI (`shadercross` binary) in
`src/main.rs` using clap. Internals:

- `spvc/` — hand-written FFI over `spirv-cross-c-shared` (`ffi.rs`) and a
  safe wrapper (`mod.rs`).
- `transpile.rs` — SPIR-V → MSL/HLSL via spvc; DXIL = spvc→HLSL→dxc.
- `dxc.rs` — invokes the `dxc` binary (temp files via `tempfile`).
- `reflect.rs` / `json.rs` — SPIR-V reflection and C-CLI-compatible JSON.
- Known differences from the C library (documented in its README): dxc is a
  CLI instead of the COM API, DXBC output is unsupported, SDL properties are
  replaced by the `Options` struct, and the C CLI's `-c`/`--cull` bug is fixed.

When porting more of the C library, preserve the exact C CLI JSON schema and
the `Options` defaults (`msl_version: "1.2.0"`, no culling).

## Conventions summary

- Root crate: edition 2024, errors as `Result<_, String>`, `Rc`/`Weak` RAII,
  single-threaded, safe public API (all `unsafe` confined to FFI call sites
  in `device.rs` mostly).
- Public API doc comments are expected on new items; keep the thin-wrapper
  philosophy — mirror SDL3 GPU API naming so SDL docs/examples translate
  directly.
- The demo example is the integration test of record; after GPU-related
  changes, build it (`cargo build --example demo`) and run it when a GPU is
  available. There are no unit tests for the main crate.
- Commit messages in this repo are currently terse; match the existing style
  unless asked to improve it.
