# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`sdl3_gs` is an early-stage, safe Rust wrapper around SDL3's GPU API (`SDL_GPU*`). It wraps the raw C FFI bindings from `sdl3-sys` (v0.6.1, targeting SDL 3.4.2) and exposes a safe, idiomatic Rust interface. The crate uses Rust edition 2024 and depends only on `sdl3-sys` and `bytemuck`.

## Commands

```sh
cargo build                        # Build the library and binary
cargo test                         # Run all tests
cargo test <name>                  # Run a specific test by name
cargo clippy                       # Lint
cargo run                          # Run the placeholder binary (src/main.rs)
cargo run --example demo           # Run the demo example
```

## Architecture

### Module overview

- **`src/lib.rs`** — Library root; declares and re-exports all modules. The crate is both a `lib` and a `bin`.
- **`src/device.rs`** — Core module (~1300 lines). Contains `Device`, all GPU resource handles (`Texture`, `Shader`, `GraphicsPipeline`, `ComputePipeline`, `GPUBuffer`, `Sampler`), pass types (`RenderPass`, `ComputePass`, `CopyPass`), `CommandBuffer`, and all creation/config structs.
- **`src/slot_map.rs`** — Handle-based resource container. `SlotMap<T>` with free-list reuse; `SlotMapRefCell<T>` wraps it in `RefCell` for interior mutability.
- **`src/window.rs`** — `Window` wraps `*mut SDL_Window` with `Drop` cleanup.
- **`src/event.rs`** — Safe `Event` enum parsed from `SDL_Event`. Provides `poll_event()` and `poll_events()` iterator.
- **`src/callbacks.rs`** — SDL3 callback-based main loop. Defines the `App` trait (`init`, `iterate`, `event`, `quit`) and `run::<T: App>()`. The `sdl3_main!` macro exports `SDL_main` for Android.
- **`src/tools.rs`** — Shader compilation utilities. `prepare_shaders()` compiles GLSL to platform-appropriate formats (SPIR-V on Linux, MSL on macOS, DXIL on Windows) using external tools (`glslc`, `glslcc`, `shadercross`).
- **`examples/demo/`** — Full working example with textured rendering, MSAA, and compute shaders.

### Design patterns

- **Handle-based resources** — GPU resources are referenced via lightweight `Copy` handles (newtypes over `i32`) backed by `SlotMap`. This avoids lifetime complexity. The special value `Texture::SWAPCHAIN` (`-7777`) refers to the current swapchain texture.
- **Interior mutability** — `Device` uses `RefCell`/`Cell` for resource containers and transient state (e.g., swapchain), allowing mutation through `&self`.
- **RAII everywhere** — `Drop` impls on `Device`, `Window`, `RenderPass`, `ComputePass`, `CopyPass`, and `CommandBuffer`. Passes auto-end on drop; command buffers auto-cancel if not submitted.
- **Deferred transfer buffer cleanup** — Upload transfer buffers are kept alive in `pending_transfer_buffers` until all in-flight command buffers complete (tracked via `cmd_buf_count: AtomicU32`).
- **Lifetime-enforced pass safety** — `CommandBuffer` borrows `&Device`; passes borrow the command buffer. This prevents submitting a command buffer while a pass is still active.
- **Error handling** — Fallible operations return `Result<T, &'static str>`. SDL errors are fetched via `SDL_GetError()`.

### SDL3 FFI conventions

- Bindings come via `use sdl3_sys as sys; use sys::*;` — SDL3 GPU symbols are under the `gpu::` module (e.g., `gpu::SDL_CreateGPUDevice`).
- All `unsafe` SDL3 calls are encapsulated internally; the public API surface is safe.
- Closures are used with `SlotMap::with()` / `with_mut()` to access resources without exposing borrows.
