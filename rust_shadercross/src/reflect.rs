//! Ports of `SDL_ShaderCross_ReflectGraphicsSPIRV` and
//! `SDL_ShaderCross_ReflectComputeSPIRV`: acquire metadata from SPIR-V bytecode.

use crate::error::{Error, Result};
use crate::spvc::{ffi, Compiler, Context};
use crate::{
    ComputePipelineMetadata, GraphicsShaderMetadata, GraphicsShaderResourceInfo, IOVarMetadata,
    IOVarType,
};
use std::ffi::CStr;

fn io_var_type(basetype: ffi::spvc_basetype) -> IOVarType {
    match basetype {
        ffi::SPVC_BASETYPE_INT8 => IOVarType::Int8,
        ffi::SPVC_BASETYPE_UINT8 => IOVarType::Uint8,
        ffi::SPVC_BASETYPE_INT16 => IOVarType::Int16,
        ffi::SPVC_BASETYPE_UINT16 => IOVarType::Uint16,
        ffi::SPVC_BASETYPE_INT32 => IOVarType::Int32,
        ffi::SPVC_BASETYPE_UINT32 => IOVarType::Uint32,
        ffi::SPVC_BASETYPE_INT64 => IOVarType::Int64,
        ffi::SPVC_BASETYPE_UINT64 => IOVarType::Uint64,
        ffi::SPVC_BASETYPE_FP16 => IOVarType::Float16,
        ffi::SPVC_BASETYPE_FP32 => IOVarType::Float32,
        ffi::SPVC_BASETYPE_FP64 => IOVarType::Float64,
        _ => IOVarType::Unknown,
    }
}

fn get_io_vars(
    compiler: &Compiler,
    resources: &[ffi::spvc_reflected_resource],
) -> Result<Vec<IOVarMetadata>> {
    let mut vars = Vec::with_capacity(resources.len());
    for resource in resources {
        let spirv_type = compiler.type_handle(resource.base_type_id);
        let vector_type = io_var_type(spirv_type.basetype());
        let vector_size = spirv_type.vector_size();
        let name = unsafe { CStr::from_ptr(resource.name) }
            .to_string_lossy()
            .into_owned();
        let location = compiler.get_decoration(resource.id, ffi::SpvDecorationLocation);
        vars.push(IOVarMetadata {
            name,
            location,
            vector_type,
            vector_size,
        });
    }
    Ok(vars)
}

fn require_set_and_binding(
    compiler: &Compiler,
    resource: &ffi::spvc_reflected_resource,
) -> Result<u32> {
    if !compiler.has_decoration(resource.id, ffi::SpvDecorationDescriptorSet)
        || !compiler.has_decoration(resource.id, ffi::SpvDecorationBinding)
    {
        return Err(Error::InvalidParameter(
            "Shader resources must have descriptor set and binding index!".into(),
        ));
    }
    Ok(compiler.get_decoration(resource.id, ffi::SpvDecorationDescriptorSet))
}

fn setup(code: &[u8]) -> Result<(Context, Compiler)> {
    let context = Context::new()?;
    let ir = context.parse_spirv(code)?;
    // Create a reflection-only compiler.
    let compiler = context.create_compiler(ffi::SPVC_BACKEND_NONE, ir)?;
    Ok((context, compiler))
}

/// Acquire graphics shader metadata from SPIR-V bytecode.
pub(crate) fn reflect_graphics_spirv(code: &[u8]) -> Result<GraphicsShaderMetadata> {
    // The context binding keeps the spirv-cross allocations alive for this scope.
    let (_context, compiler) = setup(code)?;

    let resources = {
        let active_variables = compiler.active_interface_variables()?;
        compiler.resources_for_active_variables(active_variables)?
    };

    // Combined texture-samplers. If the source is HLSL we might have separate
    // images and samplers instead.
    let num_texture_samplers;
    let mut num_separate_samplers = 0usize;
    let sampled_images = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SAMPLED_IMAGE)?;
    if !sampled_images.is_empty() {
        num_texture_samplers = sampled_images.len();
    } else {
        let separate_samplers =
            resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_SAMPLERS)?;
        num_separate_samplers = separate_samplers.len();
        num_texture_samplers = num_separate_samplers;
    }

    // Storage textures
    let mut num_storage_textures = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_IMAGE)?
        .len();

    // If source is HLSL, storage images might be marked as separate images.
    let separate_images = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_IMAGE)?;
    // The number of storage textures is the number of separate images minus
    // the number of samplers.
    num_storage_textures += separate_images.len() - num_separate_samplers;

    // Storage buffers
    let num_storage_buffers = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_BUFFER)?
        .len();

    // Uniform buffers
    let num_uniform_buffers = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_UNIFORM_BUFFER)?
        .len();

    // Inputs
    let num_inputs = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_STAGE_INPUT)?
        .len();
    // Outputs
    let num_outputs = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_STAGE_OUTPUT)?
        .len();

    // The C library re-fetches the input/output lists to fill in the
    // destination arrays; the lists are stable so a single pass suffices here.
    let inputs = get_io_vars(
        &compiler,
        resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STAGE_INPUT)?,
    )?;
    let outputs = get_io_vars(
        &compiler,
        resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STAGE_OUTPUT)?,
    )?;
    debug_assert_eq!(inputs.len(), num_inputs);
    debug_assert_eq!(outputs.len(), num_outputs);

    Ok(GraphicsShaderMetadata {
        resource_info: GraphicsShaderResourceInfo {
            num_samplers: num_texture_samplers as u32,
            num_storage_textures: num_storage_textures as u32,
            num_storage_buffers: num_storage_buffers as u32,
            num_uniform_buffers: num_uniform_buffers as u32,
        },
        inputs,
        outputs,
    })
}

/// Acquire compute pipeline metadata from SPIR-V bytecode.
pub(crate) fn reflect_compute_spirv(code: &[u8]) -> Result<ComputePipelineMetadata> {
    // The context binding keeps the spirv-cross allocations alive for this scope.
    let (_context, compiler) = setup(code)?;

    let resources = {
        let active_variables = compiler.active_interface_variables()?;
        compiler.resources_for_active_variables(active_variables)?
    };

    // Combined texture-samplers. If the source is HLSL we might have separate
    // images and samplers instead.
    let num_texture_samplers;
    let mut num_separate_samplers = 0usize;
    let sampled_images = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SAMPLED_IMAGE)?;
    if !sampled_images.is_empty() {
        num_texture_samplers = sampled_images.len();
    } else {
        let separate_samplers =
            resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_SAMPLERS)?;
        num_separate_samplers = separate_samplers.len();
        num_texture_samplers = num_separate_samplers;
    }

    // Storage textures, classified by descriptor set (0 = readonly, 1 = readwrite).
    let mut num_readonly_storage_textures = 0;
    let mut num_readwrite_storage_textures = 0;

    let storage_images = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_IMAGE)?;
    for resource in storage_images {
        let descriptor_set = require_set_and_binding(&compiler, resource)?;
        match descriptor_set {
            0 => num_readonly_storage_textures += 1,
            1 => num_readwrite_storage_textures += 1,
            _ => {
                return Err(Error::InvalidParameter(
                    "Descriptor set index for compute storage texture must be 0 or 1!".into(),
                ))
            }
        }
    }

    // If source is HLSL, readonly storage images might be marked as separate images.
    let separate_images = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_IMAGE)?;

    for resource in separate_images.iter().skip(num_separate_samplers) {
        let descriptor_set = require_set_and_binding(&compiler, resource)?;
        match descriptor_set {
            0 => num_readonly_storage_textures += 1,
            1 => num_readwrite_storage_textures += 1,
            _ => {
                return Err(Error::InvalidParameter(
                    "Descriptor set index for compute storage texture must be 0 or 1!".into(),
                ))
            }
        }
    }

    // Storage buffers
    let mut num_readonly_storage_buffers = 0;
    let mut num_readwrite_storage_buffers = 0;
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_BUFFER)? {
        let descriptor_set = require_set_and_binding(&compiler, resource)?;
        if descriptor_set != 0 && descriptor_set != 1 {
            return Err(Error::InvalidParameter(
                "Descriptor set index for compute storage buffer must be 0 or 1!".into(),
            ));
        }
        match descriptor_set {
            0 => num_readonly_storage_buffers += 1,
            _ => num_readwrite_storage_buffers += 1,
        }
    }

    // Uniform buffers
    let num_uniform_buffers = resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_UNIFORM_BUFFER)?
        .len();

    // Threadcount
    let threadcount_x = compiler.execution_mode_argument(ffi::SpvExecutionModeLocalSize, 0);
    let threadcount_y = compiler.execution_mode_argument(ffi::SpvExecutionModeLocalSize, 1);
    let threadcount_z = compiler.execution_mode_argument(ffi::SpvExecutionModeLocalSize, 2);

    Ok(ComputePipelineMetadata {
        num_samplers: num_texture_samplers as u32,
        num_readonly_storage_textures: num_readonly_storage_textures as u32,
        num_readonly_storage_buffers: num_readonly_storage_buffers as u32,
        num_readwrite_storage_textures: num_readwrite_storage_textures as u32,
        num_readwrite_storage_buffers: num_readwrite_storage_buffers as u32,
        num_uniform_buffers: num_uniform_buffers as u32,
        threadcount_x,
        threadcount_y,
        threadcount_z,
    })
}
