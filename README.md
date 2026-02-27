# sdl3_gs

A safe Rust wrapper around SDL3's GPU API.

## Goal

`sdl3_gs` aims to provide an idiomatic, safe Rust interface over SDL3's low-level GPU API (`SDL_GPU*`). Rather than working directly with raw C pointers and `unsafe` FFI calls, users get:

- **RAII resource management** — GPU resources (devices, textures, shaders, pipelines, buffers) are automatically cleaned up via `Drop` impls, preventing leaks.
- **Handle-based resource tracking** — Resources are referenced through lightweight, copyable handles backed by a slot map, avoiding lifetime complexity while keeping safety.
- **Safe public API** — All `unsafe` SDL3 calls are encapsulated internally; the public surface is safe Rust.
- **Thin abstraction** — The wrapper stays close to SDL3's GPU API design, making it easy to follow SDL3 documentation and examples while writing Rust.

The crate builds on [`sdl3-sys`](https://crates.io/crates/sdl3-sys) (v0.6.1, targeting SDL 3.4.2) for raw FFI bindings.

## Status

Early stage. The wrapper currently covers:

- Device creation and window claiming
- Texture, shader, buffer, and graphics pipeline management
- Command buffer recording with render passes and copy passes
- Swapchain texture acquisition
- Vertex/index buffer uploads via internal transfer buffers

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sdl3_gs = { git = "https://github.com/archee565/sdl3_gs.git" }
```

## License

MIT
