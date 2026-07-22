# sdl3_gs   SDL3 wrapper by Gravitysensation

A safe Rust wrapper around SDL3's GPU API.

## Goal

`sdl3_gs` aims to provide an idiomatic, safe Rust interface over SDL3's low-level GPU API (`SDL_GPU*`). Rather than working directly with raw C pointers and `unsafe` FFI calls, users get:

- **Rc-based RAII resource management** — GPU resources (devices, textures, shaders, pipelines, buffers) are backed by `Rc<Data>` and freed via `Drop` when the last handle is dropped. Each resource holds a `Weak<DeviceInner>` back-reference, preventing cycles while allowing graceful cleanup even if the device is destroyed first.
- **Safe public API** — All `unsafe` SDL3 calls are encapsulated internally; the public surface is safe Rust.
- **Thin abstraction** — The wrapper stays close to SDL3's GPU API design, making it easy to follow SDL3 documentation and examples while writing Rust.

The crate builds on [`sdl3-sys`](https://crates.io/crates/sdl3-sys) for raw FFI bindings.

## Modules

| Module | Description |
|---|---|
| `device` | GPU device, all resource types (Texture, Shader, GPUBuffer, GraphicsPipeline, ComputePipeline, Sampler, Fence, GPUTransferBuffer), CommandBuffer with render/copy/compute passes, and upload/download helpers |
| `window` | Safe SDL3 window wrapper with `Drop` |
| `event` | Typed event enum (Quit, KeyDown/Up, Mouse, Window, etc.) with polling and iterator API |
| `callbacks` | SDL3 main-callbacks pattern via an `App` trait + `sdl3_main!` macro for cross-platform entry |
| `properties` | Safe wrapper around `SDL_PropertiesID` with typed getters/setters and owned vs. borrowed semantics |
| `tools` | Build-time GLSL-to-SPIR-V compilation helpers |

## Status

Active development. The wrapper currently covers:

- Device creation, window claiming, swapchain management
- Texture, shader, buffer, graphics pipeline, compute pipeline, sampler, fence, and transfer buffer management
- Command buffer lifecycle (acquire, submit with/without fence, cancel on drop)
- Render passes (vertex/index buffers, push uniforms, viewport/scissor, storage bindings, draw indirect)
- Compute passes (storage textures/buffers, samplers, push uniforms, dispatch indirect)
- Copy passes (buffer-to-buffer)
- Blit operations
- Texture and buffer uploads via cached internal transfer buffers
- Buffer downloads with fence synchronization and typed `Pod` helpers
- MSAA render targets and resolves
- Demo example showing textured MSAA rendering and compute dispatch

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sdl3_gs = { git = "https://github.com/archee565/sdl3_gs.git" }
```

## License

MIT
