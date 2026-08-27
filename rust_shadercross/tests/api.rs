//! End-to-end tests for the shadercross library API.
//!
//! These require a SPIR-V-enabled `dxc` binary on `PATH` (or in
//! `SHADERCROSS_DXC`) and `libspirv-cross-c-shared` available at link time.

use shadercross::{
    compile_dxil_from_hlsl, compile_dxil_from_spirv, compile_spirv_from_hlsl, json,
    reflect_compute_spirv, reflect_graphics_spirv, transpile_hlsl_from_spirv,
    transpile_msl_from_spirv, ComputePipelineMetadata, HlslInfo, Options, ShaderStage, SpirvInfo,
};

const VERTEX_HLSL: &str = include_str!("../../test/shaders/simple.vert.hlsl");

const COMPUTE_HLSL: &str = r#"
Texture2D<float4> ReadOnlyTex : register(t0);
StructuredBuffer<float4> ReadOnlyBuf : register(t1);
RWTexture2D<float4> ReadWriteTex : register(u0, space1);
RWStructuredBuffer<float4> ReadWriteBuf : register(u1, space1);
cbuffer Uniforms : register(b0, space2)
{
    float4 Scale;
};

[numthreads(8, 4, 2)]
void main(uint3 id : SV_DispatchThreadID)
{
    float4 value = ReadOnlyBuf[id.x] * ReadOnlyTex.Load(int3(id.xy, 0)) * Scale;
    ReadWriteTex[id.xy] = value * ReadWriteBuf[id.x];
}
"#;

fn options() -> Options {
    Options::default()
}

#[test]
fn hlsl_to_spirv() {
    let options = options();
    let info = HlslInfo {
        source: VERTEX_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &[],
        shader_stage: ShaderStage::Vertex,
        options: &options,
    };
    let spirv = compile_spirv_from_hlsl(&info).expect("SPIR-V compilation failed");
    // SPIR-V magic number.
    assert_eq!(&spirv[0..4], &[0x03, 0x02, 0x23, 0x07]);
    assert_eq!(spirv.len() % 4, 0);
}

fn vertex_spirv() -> Vec<u8> {
    let options = options();
    let info = HlslInfo {
        source: VERTEX_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &[],
        shader_stage: ShaderStage::Vertex,
        options: &options,
    };
    compile_spirv_from_hlsl(&info).expect("SPIR-V compilation failed")
}

#[test]
fn spirv_to_msl() {
    let spirv = vertex_spirv();
    let options = options();
    let msl = transpile_msl_from_spirv(&SpirvInfo {
        bytecode: &spirv,
        entrypoint: "main",
        shader_stage: ShaderStage::Vertex,
        options: &options,
    })
    .expect("MSL transpilation failed");
    // Metal does not allow "main"; the entrypoint must be cleansed.
    assert!(
        msl.contains("main0"),
        "expected cleansed entrypoint in:\n{msl}"
    );
    // The uniform block (descriptor set 1, binding 0) lands on buffer 0.
    assert!(
        msl.contains("[[buffer(0)]]"),
        "expected UBO remap in:\n{msl}"
    );
}

#[test]
fn spirv_to_hlsl() {
    let spirv = vertex_spirv();
    let options = options();
    let hlsl = transpile_hlsl_from_spirv(&SpirvInfo {
        bytecode: &spirv,
        entrypoint: "main",
        shader_stage: ShaderStage::Vertex,
        options: &options,
    })
    .expect("HLSL transpilation failed");
    assert!(hlsl.contains("cbuffer"), "expected cbuffer in:\n{hlsl}");
}

#[test]
fn spirv_to_dxil() {
    let spirv = vertex_spirv();
    let options = options();
    let dxil = compile_dxil_from_spirv(&SpirvInfo {
        bytecode: &spirv,
        entrypoint: "main",
        shader_stage: ShaderStage::Vertex,
        options: &options,
    })
    .expect("DXIL compilation failed");
    // DXIL containers reuse the DXBC container magic.
    assert_eq!(&dxil[0..4], b"DXBC");
}

#[test]
fn hlsl_to_dxil_roundtrip() {
    let options = options();
    let info = HlslInfo {
        source: VERTEX_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &[],
        shader_stage: ShaderStage::Vertex,
        options: &options,
    };
    let dxil = compile_dxil_from_hlsl(&info).expect("DXIL compilation failed");
    assert_eq!(&dxil[0..4], b"DXBC");
}

#[test]
fn hlsl_to_dxil_skip_roundtrip() {
    let options = Options {
        skip_spirv_roundtrip: true,
        ..Options::default()
    };
    let info = HlslInfo {
        source: VERTEX_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &[],
        shader_stage: ShaderStage::Vertex,
        options: &options,
    };
    let dxil = compile_dxil_from_hlsl(&info).expect("DXIL compilation failed");
    assert_eq!(&dxil[0..4], b"DXBC");
}

#[test]
fn reflect_graphics() {
    let spirv = vertex_spirv();
    let metadata = reflect_graphics_spirv(&spirv).expect("graphics reflection failed");
    assert_eq!(metadata.resource_info.num_uniform_buffers, 1);
    assert_eq!(metadata.resource_info.num_samplers, 0);
    assert_eq!(metadata.inputs.len(), 1);
    assert_eq!(metadata.inputs[0].name, "in.var.TEXCOORD0");
    assert_eq!(metadata.inputs[0].vector_size, 3);
    assert_eq!(metadata.inputs[0].location, 0);
    // SV_Position is a builtin and does not show up as a stage output.
    assert_eq!(metadata.outputs.len(), 1);
    assert_eq!(metadata.outputs[0].name, "out.var.TEXCOORD0");
    assert_eq!(
        metadata.outputs[0].vector_type,
        shadercross::IOVarType::Float32
    );
}

#[test]
fn reflect_compute() {
    let options = options();
    let info = HlslInfo {
        source: COMPUTE_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &[],
        shader_stage: ShaderStage::Compute,
        options: &options,
    };
    let spirv = compile_spirv_from_hlsl(&info).expect("SPIR-V compilation failed");

    let metadata = reflect_compute_spirv(&spirv).expect("compute reflection failed");
    assert_eq!(metadata.num_samplers, 0);
    assert_eq!(metadata.num_readonly_storage_textures, 1);
    assert_eq!(metadata.num_readonly_storage_buffers, 1);
    assert_eq!(metadata.num_readwrite_storage_textures, 1);
    assert_eq!(metadata.num_readwrite_storage_buffers, 1);
    assert_eq!(metadata.num_uniform_buffers, 1);
    assert_eq!(metadata.threadcount_x, 8);
    assert_eq!(metadata.threadcount_y, 4);
    assert_eq!(metadata.threadcount_z, 2);

    // Canonical SDL compute layout must remap without collisions.
    let msl = transpile_msl_from_spirv(&SpirvInfo {
        bytecode: &spirv,
        entrypoint: "main",
        shader_stage: ShaderStage::Compute,
        options: &options,
    })
    .expect("MSL transpilation failed");
    assert!(msl.contains("Uniforms [[buffer(0)]]"), "{msl}");
    assert!(msl.contains("ReadOnlyBuf [[buffer(1)]]"), "{msl}");
    assert!(msl.contains("ReadWriteBuf [[buffer(2)]]"), "{msl}");
    assert!(msl.contains("ReadOnlyTex [[texture(0)]]"), "{msl}");
    assert!(msl.contains("ReadWriteTex [[texture(1)]]"), "{msl}");
}

#[test]
fn json_output_format() {
    let metadata = ComputePipelineMetadata {
        num_samplers: 1,
        num_readonly_storage_textures: 2,
        num_readonly_storage_buffers: 3,
        num_readwrite_storage_textures: 4,
        num_readwrite_storage_buffers: 5,
        num_uniform_buffers: 6,
        threadcount_x: 8,
        threadcount_y: 4,
        threadcount_z: 2,
    };
    assert_eq!(
        json::write_compute_reflect_json(&metadata),
        "{ \"samplers\": 1, \"readonly_storage_textures\": 2, \"readonly_storage_buffers\": 3, \
         \"readwrite_storage_textures\": 4, \"readwrite_storage_buffers\": 5, \"uniform_buffers\": 6, \
         \"threadcount_x\": 8, \"threadcount_y\": 4, \"threadcount_z\": 2 }\n"
    );
}

#[test]
fn hlsl_defines_and_include_dir() {
    let options = options();
    let defines = [shadercross::HlslDefine {
        name: "BREAK_SHADER".to_owned(),
        value: None,
    }];

    // The test shader chokes when BREAK_SHADER is defined.
    let info = HlslInfo {
        source: VERTEX_HLSL,
        entrypoint: "main",
        include_dir: None,
        defines: &defines,
        shader_stage: ShaderStage::Vertex,
        options: &options,
    };
    assert!(compile_spirv_from_hlsl(&info).is_err());

    // ...and compiles fine without it.
    let info = HlslInfo {
        defines: &[],
        ..info
    };
    assert!(compile_spirv_from_hlsl(&info).is_ok());
}
