Replace the `shadercross` CLI invocations in `src/shader_build.rs` with library calls into the local `rust_shadercross` crate. GLSL→SPIR-V stays on `glslc` (the crate has no GLSL frontend); only the SPIR-V→{DXIL, MSL, JSON} stage changes.

## 1. Wire up the dependency (`Cargo.toml`)
- Add `rust_shadercross` to `[workspace] members` (crate name is `shadercross`).
- `[build-dependencies]`: `shadercross = { path = "rust_shadercross" }` (build.rs always compiles shaders, mirroring the unconditional `rayon` build-dep).
- `[dependencies]`: `shadercross = { path = "rust_shadercross", optional = true }`, and extend the existing feature: `shader-compiler = ["dep:rayon", "dep:shadercross"]` — mirrors how `rayon` is already gated for the lib copy of the module.

## 2. `src/shader_build.rs` changes
- Delete the CLI plumbing: the `shadercross --help` availability probe (line 69), the `warned_no_shadercross` atomic + warning block (lines 71, 101–114), and the "input changed but can't regenerate" stale-output branch (lines 134–143) — the converter is now always available.
- Replace the `"vertex"/"fragment"/"compute"` string with `shadercross::ShaderStage`, mapped from the file extension at the call site.
- Rewrite `convert_spirv_to_formats` to:
  - `fs::read` the `.spv` bytecode and build `SpirvInfo { bytecode, entrypoint: "main", shader_stage, options: &Options::default() }` (entry is `main` because preprocessing renames entries; defaults match the old CLI flags: no `-c`, MSL 1.2.0).
  - DXIL: `shadercross::compile_dxil_from_spirv(&info)` → write bytes to the `.dxil` path.
  - MSL: `shadercross::transpile_msl_from_spirv(&info)` → write the string to the `.metal` path.
  - JSON: compute shaders use `reflect_compute_spirv` + `json::write_compute_reflect_json`; graphics use `reflect_graphics_spirv` + `json::write_graphics_reflect_json` → write to the `.json` path.
  - Keep the existing `cargo:rerun-if-changed` prints and the eprintln+panic error handling on `Err` (matches current CLI-failure behavior). The crate creates a spirv-cross context per call, so the existing `rayon` parallelism stays safe.

## 3. Verification
- `cargo check` / `cargo build` with `shader-compiler` enabled.
- Force regeneration (clear `target/shaders`) and confirm outputs: `.dxil` starts with the `DXBC` magic, `.metal` is MSL text, `.json` parses and matches the old CLI schema (the crate's JSON writer replicates it exactly).

## Notes / out of scope
- `src/tools.rs` also shells out to the `shadercross` CLI at runtime — left untouched per the request scope.
- New link-time requirement when the feature is on: `libspirv-cross-c-shared` (Vulkan SDK, located by the crate's build.rs) and `dxc` on PATH for DXIL — same tools the old CLI needed.