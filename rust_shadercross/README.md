# SDL_shadercross — Rust port

A Rust port of SDL_shadercross: a shader cross-compiler for HLSL and SPIR-V,
producing SPIR-V, MSL, HLSL and DXIL. It contains the `shadercross` library
crate and the `shadercross` command line tool (a port of the C CLI in
`src/cli.c`).

## Dependencies

Two system dependencies are expected to be installed:

- **dxc** — the DirectX Shader Compiler, invoked as a command line tool.
  SPIR-V output requires a dxc build with SPIR-V codegen, such as the one
  shipped with the [Vulkan SDK](https://vulkan.lunarg.com/). The binary is
  looked up as `dxc` on `PATH`, or the path from the `SHADERCROSS_DXC`
  environment variable.
- **spirv-cross** — the C API shared library (`libspirv-cross-c-shared`) as
  installed by the Vulkan SDK (on Linux distro installs it may be named
  `libspirv-cross-c-shared.so.0`). The build script locates it via
  `SHADERCROSS_SPIRV_CROSS_DIR`, `VULKAN_SDK`, pkg-config, or the default
  linker search path, and emits an rpath so the binary finds it at runtime.

## Differences from the C library

- `dxc` is invoked from the command line instead of linking `dxcompiler` and
  using the COM-based `IDxcCompiler3` API.
- DXBC (shader model 5) output is not supported; the C library uses FXC
  (`d3dcompiler_47` / vkd3d-utils) for that. DXBC destinations are rejected.
- The SDL property mechanism is replaced by the `shadercross::Options` struct,
  and the SDL GPU runtime shader creation functions
  (`SDL_ShaderCross_CompileGraphicsShaderFromSPIRV` /
  `SDL_ShaderCross_CompileComputePipelineFromSPIRV`) are omitted along with
  the SDL dependency entirely.
- The CLI bug in `src/cli.c` (line 676) where the `-c`/`--cull` flag set the
  debug-enable property instead of the cull property is fixed.

## Building

```sh
cargo build --release
```

The binary is at `target/release/shadercross`; the library is available as
the `shadercross` crate.

## CLI usage

```sh
shadercross shader.vert.hlsl -o shader.vert.spv   # HLSL -> SPIR-V
shadercross shader.vert.spv -o shader.msl         # SPIR-V -> MSL
shadercross shader.vert.spv -o shader.hlsl        # SPIR-V -> HLSL
shadercross shader.vert.spv -o shader.dxil        # SPIR-V -> DXIL
shadercross shader.vert.hlsl -o shader.dxil       # HLSL -> DXIL (SPIR-V round trip)
shadercross shader.vert.hlsl -o shader.json       # reflection metadata as JSON
```

Run `shadercross -h` for the full option list; source format, destination
format and shader stage are inferred from file names when possible.

## Library example

```rust
use shadercross::{compile_spirv_from_hlsl, transpile_msl_from_spirv, HlslInfo, Options, ShaderStage, SpirvInfo};

let options = Options::default();
let hlsl = HlslInfo {
    source: "...",
    entrypoint: "main",
    include_dir: None,
    defines: &[],
    shader_stage: ShaderStage::Vertex,
    options: &options,
};
let spirv = compile_spirv_from_hlsl(&hlsl)?;
let msl = transpile_msl_from_spirv(&SpirvInfo {
    bytecode: &spirv,
    entrypoint: "main",
    shader_stage: ShaderStage::Vertex,
    options: &options,
})?;
```

## Tests

`cargo test` runs the integration test suite in `tests/api.rs`. The tests
invoke `dxc` and therefore require it to be installed.
