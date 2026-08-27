//! SDL_shadercross, ported to Rust: a shader cross-compiler for HLSL and
//! SPIR-V, targeting SPIR-V, MSL, HLSL and DXIL.
//!
//! Differences from the C library:
//! - `dxc` is invoked as a command line tool (set `SHADERCROSS_DXC` to
//!   override the binary); SPIR-V output requires a dxc build with SPIR-V
//!   codegen, such as the one shipped with the Vulkan SDK.
//! - spirv-cross is linked as the shared library installed by the Vulkan SDK.
//! - DXBC (shader model 5) output is not supported; the C library uses FXC
//!   for that, which is out of scope here.
//! - The SDL property mechanism is replaced by [`Options`], and the SDL GPU
//!   runtime shader creation functions are omitted.

pub mod error;
pub mod json;

mod dxc;
mod reflect;
mod spvc;
mod transpile;

pub use error::{Error, Result};

/// The shader stage of the source shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

/// The base type of a shader input/output variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IOVarType {
    Unknown,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float16,
    Float32,
    Float64,
}

/// Metadata about a shader input/output variable.
#[derive(Clone, Debug)]
pub struct IOVarMetadata {
    /// The UTF-8 name of the variable.
    pub name: String,
    /// The location of the variable.
    pub location: u32,
    /// The vector type of the variable.
    pub vector_type: IOVarType,
    /// The number of components in the vector type of the variable.
    pub vector_size: u32,
}

/// The resource counts of a graphics shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphicsShaderResourceInfo {
    /// The number of samplers defined in the shader.
    pub num_samplers: u32,
    /// The number of storage textures defined in the shader.
    pub num_storage_textures: u32,
    /// The number of storage buffers defined in the shader.
    pub num_storage_buffers: u32,
    /// The number of uniform buffers defined in the shader.
    pub num_uniform_buffers: u32,
}

/// Metadata of a graphics shader, as produced by reflection.
#[derive(Clone, Debug)]
pub struct GraphicsShaderMetadata {
    /// Sub-struct containing the resource info of the shader.
    pub resource_info: GraphicsShaderResourceInfo,
    /// The inputs defined in the shader.
    pub inputs: Vec<IOVarMetadata>,
    /// The outputs defined in the shader.
    pub outputs: Vec<IOVarMetadata>,
}

/// Metadata of a compute pipeline, as produced by reflection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputePipelineMetadata {
    /// The number of samplers defined in the shader.
    pub num_samplers: u32,
    /// The number of readonly storage textures defined in the shader.
    pub num_readonly_storage_textures: u32,
    /// The number of readonly storage buffers defined in the shader.
    pub num_readonly_storage_buffers: u32,
    /// The number of read-write storage textures defined in the shader.
    pub num_readwrite_storage_textures: u32,
    /// The number of read-write storage buffers defined in the shader.
    pub num_readwrite_storage_buffers: u32,
    /// The number of uniform buffers defined in the shader.
    pub num_uniform_buffers: u32,
    /// The number of threads in the X dimension.
    pub threadcount_x: u32,
    /// The number of threads in the Y dimension.
    pub threadcount_y: u32,
    /// The number of threads in the Z dimension.
    pub threadcount_z: u32,
}

/// Compiler options, replacing the SDL property keys of the C library.
#[derive(Clone, Debug)]
pub struct Options {
    /// Allows debug info to be emitted when relevant. Should only be used
    /// with debugging tools like RenderDoc.
    pub debug: bool,
    /// A UTF-8 name to be used with the shader. Relevant for use with
    /// debugging tools like RenderDoc.
    pub debug_name: Option<String>,
    /// When true, indicates that the compiler should cull unused shader
    /// resources. This behavior is disabled by default.
    pub cull_unused_bindings: bool,
    /// Generates PSSL-compatible HLSL when transpiling to HLSL.
    pub pssl_compatibility: bool,
    /// The MSL version to emit, formatted as "major.minor.patch".
    /// Defaults to "1.2.0".
    pub msl_version: String,
    /// When true, the SPIRV roundtrip of HLSL-to-DXIL compilation is skipped.
    /// This behavior is disabled by default. Do not use this if your shader
    /// uses Structured Buffers.
    pub skip_spirv_roundtrip: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            debug: false,
            debug_name: None,
            cull_unused_bindings: false,
            pssl_compatibility: false,
            msl_version: "1.2.0".to_owned(),
            skip_spirv_roundtrip: false,
        }
    }
}

/// A HLSL preprocessor define.
#[derive(Clone, Debug)]
pub struct HlslDefine {
    pub name: String,
    /// An optional value for the define. If omitted, the define is treated
    /// as equal to 1.
    pub value: Option<String>,
}

/// A description of SPIR-V bytecode to transpile or reflect.
#[derive(Clone, Copy, Debug)]
pub struct SpirvInfo<'a> {
    /// The SPIR-V bytecode.
    pub bytecode: &'a [u8],
    /// The entrypoint function name for the shader in UTF-8.
    pub entrypoint: &'a str,
    /// The shader stage to transpile the shader with.
    pub shader_stage: ShaderStage,
    pub options: &'a Options,
}

/// A description of HLSL source code to compile.
#[derive(Clone, Copy, Debug)]
pub struct HlslInfo<'a> {
    /// The HLSL source code for the shader.
    pub source: &'a str,
    /// The entrypoint function name for the shader in UTF-8.
    pub entrypoint: &'a str,
    /// The include directory for shader code. Optional.
    pub include_dir: Option<&'a std::path::Path>,
    /// An array of defines. Optional.
    pub defines: &'a [HlslDefine],
    /// The shader stage to compile the shader with.
    pub shader_stage: ShaderStage,
    pub options: &'a Options,
}

/// The shader formats that can be produced from SPIR-V source:
/// SPIR-V and MSL are always available, DXIL requires dxc.
pub fn spirv_shader_formats() -> &'static [&'static str] {
    &["SPIRV", "MSL", "DXIL"]
}

/// The shader formats that can be produced from HLSL source. Requires dxc.
pub fn hlsl_shader_formats() -> &'static [&'static str] {
    &["SPIRV", "MSL", "HLSL", "DXIL"]
}

/// Transpile to MSL code from SPIR-V code.
///
/// The optional [`Options::msl_version`] specifies the MSL version that
/// should be emitted (defaults to 1.2.0).
pub fn transpile_msl_from_spirv(info: &SpirvInfo) -> Result<String> {
    let output = transpile::transpile_from_spirv(
        spvc::ffi::SPVC_BACKEND_MSL,
        0,
        info.shader_stage,
        info.bytecode,
        info.entrypoint,
        info.options,
    )?;
    Ok(output.source)
}

/// Transpile to HLSL code from SPIR-V code.
///
/// Set [`Options::pssl_compatibility`] to generate a PSSL-compatible shader.
pub fn transpile_hlsl_from_spirv(info: &SpirvInfo) -> Result<String> {
    let shader_model = if info.options.pssl_compatibility {
        50
    } else {
        60
    };
    let output = transpile::transpile_from_spirv(
        spvc::ffi::SPVC_BACKEND_HLSL,
        shader_model,
        info.shader_stage,
        info.bytecode,
        info.entrypoint,
        info.options,
    )?;
    Ok(output.source)
}

/// Compile DXIL bytecode from SPIR-V code.
pub fn compile_dxil_from_spirv(info: &SpirvInfo) -> Result<Vec<u8>> {
    let output = transpile::transpile_from_spirv(
        spvc::ffi::SPVC_BACKEND_HLSL,
        60,
        info.shader_stage,
        info.bytecode,
        info.entrypoint,
        info.options,
    )?;

    dxc::compile_using_dxc(
        &HlslInfo {
            source: &output.source,
            entrypoint: &output.cleansed_entrypoint,
            include_dir: None,
            defines: &[],
            shader_stage: info.shader_stage,
            options: info.options,
        },
        false,
    )
}

/// Compile to SPIR-V bytecode from HLSL code.
pub fn compile_spirv_from_hlsl(info: &HlslInfo) -> Result<Vec<u8>> {
    dxc::compile_using_dxc(info, true)
}

/// Compile to DXIL bytecode from HLSL code, via a SPIR-V round trip.
///
/// Set [`Options::skip_spirv_roundtrip`] to skip the round trip; do not do
/// that if your shader uses Structured Buffers.
pub fn compile_dxil_from_hlsl(info: &HlslInfo) -> Result<Vec<u8>> {
    if info.options.skip_spirv_roundtrip {
        return dxc::compile_using_dxc(info, false);
    }

    // Roundtrip to SPIR-V to support things like Structured Buffers.
    let spirv = compile_spirv_from_hlsl(info)?;
    let spirv_info = SpirvInfo {
        bytecode: &spirv,
        entrypoint: info.entrypoint,
        shader_stage: info.shader_stage,
        options: info.options,
    };
    let translated_source = transpile_hlsl_from_spirv(&spirv_info)?;

    let translated_info = HlslInfo {
        source: &translated_source,
        ..*info
    };
    dxc::compile_using_dxc(&translated_info, false)
}

/// Reflect graphics shader info from SPIR-V code. If your shader source is
/// HLSL, you should obtain SPIR-V bytecode from [`compile_spirv_from_hlsl`].
pub fn reflect_graphics_spirv(bytecode: &[u8]) -> Result<GraphicsShaderMetadata> {
    reflect::reflect_graphics_spirv(bytecode)
}

/// Reflect compute pipeline info from SPIR-V code. If your shader source is
/// HLSL, you should obtain SPIR-V bytecode from [`compile_spirv_from_hlsl`].
pub fn reflect_compute_spirv(bytecode: &[u8]) -> Result<ComputePipelineMetadata> {
    reflect::reflect_compute_spirv(bytecode)
}
