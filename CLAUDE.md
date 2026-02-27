# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`sdl3_gs` is an early-stage, safe Rust wrapper around SDL3's GPU API (`SDL_GPU*`). It wraps the raw C FFI bindings from `sdl3-sys` (v0.6.1, targeting SDL 3.4.2) and exposes a safe, idiomatic Rust interface.

## Commands

```sh
cargo build          # Build the library and binary
cargo test           # Run all tests
cargo test <name>    # Run a specific test by name
cargo clippy         # Lint
cargo run            # Run the placeholder binary (src/main.rs)
```

## Architecture

- **`src/lib.rs`** — Library root; declares modules. The crate is both a `lib` and a `bin` (see `Cargo.toml`).
- **`src/device.rs`** — `Device` wraps `*mut SDL_GPUDevice` with RAII cleanup via `Drop` (calls `SDL_DestroyGPUDevice`).
- **`src/main.rs`** — Placeholder binary entry point.

### Design patterns

- All SDL3 GPU types are wrapped in Rust structs that own the raw pointer.
- `Drop` impls are used for all resource cleanup (RAII).
- Raw SDL3 calls live inside `unsafe` blocks; the public API surface should be safe.
- Bindings come via `use sdl3_sys as sys; use sys::*;` — SDL3 GPU symbols are under the `gpu::` module (e.g., `gpu::SDL_CreateGPUDevice`).
