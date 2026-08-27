//! JSON reflection output for the CLI, replicating the exact schema and
//! formatting of `write_graphics_reflect_json`/`write_compute_reflect_json`
//! in the C tool.

use crate::{ComputePipelineMetadata, GraphicsShaderMetadata, IOVarMetadata, IOVarType};

fn io_var_type_to_string(io_var_type: IOVarType, vector_size: u32) -> &'static str {
    match (io_var_type, vector_size) {
        (IOVarType::Int8, 1) => "byte",
        (IOVarType::Int8, 2) => "byte2",
        (IOVarType::Int8, 3) => "byte3",
        (IOVarType::Int8, 4) => "byte4",
        (IOVarType::Uint8, 1) => "ubyte",
        (IOVarType::Uint8, 2) => "ubyte2",
        (IOVarType::Uint8, 3) => "ubyte3",
        (IOVarType::Uint8, 4) => "ubyte4",
        (IOVarType::Int16, 1) => "short",
        (IOVarType::Int16, 2) => "short2",
        (IOVarType::Int16, 3) => "short3",
        (IOVarType::Int16, 4) => "short4",
        (IOVarType::Uint16, 1) => "ushort",
        (IOVarType::Uint16, 2) => "ushort2",
        (IOVarType::Uint16, 3) => "ushort3",
        (IOVarType::Uint16, 4) => "ushort4",
        (IOVarType::Int32, 1) => "int",
        (IOVarType::Int32, 2) => "int2",
        (IOVarType::Int32, 3) => "int3",
        (IOVarType::Int32, 4) => "int4",
        (IOVarType::Uint32, 1) => "uint",
        (IOVarType::Uint32, 2) => "uint2",
        (IOVarType::Uint32, 3) => "uint3",
        (IOVarType::Uint32, 4) => "uint4",
        (IOVarType::Int64, 1) => "long",
        (IOVarType::Int64, 2) => "long2",
        (IOVarType::Int64, 3) => "long3",
        (IOVarType::Int64, 4) => "long4",
        (IOVarType::Uint64, 1) => "ulong",
        (IOVarType::Uint64, 2) => "ulong2",
        (IOVarType::Uint64, 3) => "ulong3",
        (IOVarType::Uint64, 4) => "ulong4",
        (IOVarType::Float16, 1) => "half",
        (IOVarType::Float16, 2) => "half2",
        (IOVarType::Float16, 3) => "half3",
        (IOVarType::Float16, 4) => "half4",
        (IOVarType::Float32, 1) => "float",
        (IOVarType::Float32, 2) => "float2",
        (IOVarType::Float32, 3) => "float3",
        (IOVarType::Float32, 4) => "float4",
        (IOVarType::Float64, 1) => "double",
        (IOVarType::Float64, 2) => "double2",
        (IOVarType::Float64, 3) => "double3",
        (IOVarType::Float64, 4) => "double4",
        _ => {
            eprintln!(
                "Unknown IO variable type: vector_type={} vector_size={}",
                io_var_type as u32, vector_size
            );
            "unknown"
        }
    }
}

fn io_var_to_json(var: &IOVarMetadata, is_last: bool) -> String {
    format!(
        "{{ \"name\": \"{}\", \"type\": \"{}\", \"location\": {} }}{}",
        var.name,
        io_var_type_to_string(var.vector_type, var.vector_size),
        var.location,
        if is_last { "" } else { ", " }
    )
}

/// Formats graphics shader metadata as JSON, identical to the C tool.
pub fn write_graphics_reflect_json(info: &GraphicsShaderMetadata) -> String {
    let mut json = format!(
        "{{ \"samplers\": {}, \"storage_textures\": {}, \"storage_buffers\": {}, \"uniform_buffers\": {}, ",
        info.resource_info.num_samplers,
        info.resource_info.num_storage_textures,
        info.resource_info.num_storage_buffers,
        info.resource_info.num_uniform_buffers
    );

    json.push_str("\"inputs\": [");
    for (index, input) in info.inputs.iter().enumerate() {
        json.push_str(&io_var_to_json(input, index + 1 == info.inputs.len()));
    }
    json.push_str("], ");

    json.push_str("\"outputs\": [");
    for (index, output) in info.outputs.iter().enumerate() {
        json.push_str(&io_var_to_json(output, index + 1 == info.outputs.len()));
    }
    json.push_str("] }\n");

    json
}

/// Formats compute pipeline metadata as JSON, identical to the C tool.
pub fn write_compute_reflect_json(info: &ComputePipelineMetadata) -> String {
    format!(
        "{{ \"samplers\": {}, \"readonly_storage_textures\": {}, \"readonly_storage_buffers\": {}, \"readwrite_storage_textures\": {}, \"readwrite_storage_buffers\": {}, \"uniform_buffers\": {}, \"threadcount_x\": {}, \"threadcount_y\": {}, \"threadcount_z\": {} }}\n",
        info.num_samplers,
        info.num_readonly_storage_textures,
        info.num_readonly_storage_buffers,
        info.num_readwrite_storage_textures,
        info.num_readwrite_storage_buffers,
        info.num_uniform_buffers,
        info.threadcount_x,
        info.threadcount_y,
        info.threadcount_z
    )
}
