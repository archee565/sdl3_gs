//! Port of `SDL_ShaderCross_INTERNAL_TranspileFromSPIRV`: transpiles SPIR-V to
//! MSL or HLSL via spirv-cross, including the Metal resource binding remap.

use crate::error::{Error, Result};
use crate::spvc::{ffi, Compiler, Context};
use crate::{Options, ShaderStage};

pub(crate) struct TranspileOutput {
    pub source: String,
    pub cleansed_entrypoint: String,
}

/// Parses an "X.Y.Z" version string into `major*10000 + minor*100 + patch`,
/// as used by spirv-cross' MSL version option.
fn parse_version_number(string: &str) -> Option<u32> {
    let parts: Vec<&str> = string.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let major: u32 = parts[0].trim().parse().ok()?;
    let minor: u32 = parts[1].trim().parse().ok()?;
    let patch: u32 = parts[2].trim().parse().ok()?;
    Some(major * 10000 + minor * 100 + patch)
}

pub(crate) fn transpile_from_spirv(
    backend: ffi::spvc_backend,
    shader_model: u32, // only used for HLSL
    shader_stage: ShaderStage,
    code: &[u8],
    entrypoint: &str,
    options: &Options,
) -> Result<TranspileOutput> {
    let context = Context::new()?;
    let ir = context.parse_spirv(code)?;
    let compiler = context.create_compiler(backend, ir)?;
    let compiler_options = compiler.create_options()?;

    if backend == ffi::SPVC_BACKEND_HLSL {
        compiler_options.set_uint(ffi::SPVC_COMPILER_OPTION_HLSL_SHADER_MODEL, shader_model)?;
        compiler_options.set_uint(
            ffi::SPVC_COMPILER_OPTION_HLSL_NONWRITABLE_UAV_TEXTURE_AS_SRV,
            1,
        )?;
        compiler_options.set_uint(
            ffi::SPVC_COMPILER_OPTION_HLSL_FLATTEN_MATRIX_VERTEX_INPUT_SEMANTICS,
            1,
        )?;
        compiler_options.set_bool(
            ffi::SPVC_COMPILER_OPTION_HLSL_USE_ENTRY_POINT_NAME,
            !options.pssl_compatibility,
        )?;
        compiler_options.set_bool(ffi::SPVC_COMPILER_OPTION_HLSL_POINT_SIZE_COMPAT, true)?;
    }

    let execution_model = match shader_stage {
        ShaderStage::Vertex => ffi::SpvExecutionModelVertex,
        ShaderStage::Fragment => ffi::SpvExecutionModelFragment,
        ShaderStage::Compute => {
            if backend == ffi::SPVC_BACKEND_HLSL {
                ffi::SpvExecutionModelKernel
            } else {
                ffi::SpvExecutionModelGLCompute
            }
        }
    };

    if backend == ffi::SPVC_BACKEND_MSL {
        let msl_version = parse_version_number(&options.msl_version).ok_or_else(|| {
            Error::InvalidParameter(format!(
                "failed to parse MSL version string \"{}\"",
                options.msl_version
            ))
        })?;
        compiler_options.set_uint(ffi::SPVC_COMPILER_OPTION_MSL_VERSION, msl_version)?;
    }

    // MSL doesn't have descriptor sets, so we have to set up index remapping.
    if backend == ffi::SPVC_BACKEND_MSL && shader_stage != ShaderStage::Compute {
        let bindings = graphics_msl_bindings(&compiler, execution_model)?;
        for binding in &bindings.texture_bindings {
            // Textures come first so we can just use the binding slot.
            let mut binding = *binding;
            binding.msl_texture = binding.binding;
            binding.msl_sampler = binding.binding;
            compiler.add_msl_resource_binding(&binding)?;
        }

        let uniform_buffer_count = bindings
            .buffer_bindings
            .iter()
            .filter(|b| b.desc_set == 1 || b.desc_set == 3)
            .count() as u32;

        for buffer in &bindings.buffer_bindings {
            let mut binding = *buffer;
            if binding.desc_set == 1 || binding.desc_set == 3 {
                // Uniform buffers are alone in the descriptor set.
                binding.msl_buffer = binding.binding;
            } else {
                // Subtract the texture count because the textures precede the
                // storage buffers in the descriptor set.
                binding.msl_buffer = uniform_buffer_count.wrapping_add(
                    binding
                        .binding
                        .wrapping_sub(bindings.texture_bindings.len() as u32),
                );
            }
            compiler.add_msl_resource_binding(&binding)?;
        }
    }

    if backend == ffi::SPVC_BACKEND_MSL && shader_stage == ShaderStage::Compute {
        let bindings = compute_msl_bindings(&compiler, execution_model)?;

        let mut readonly_texture_count: u32 = 0;
        let mut readwrite_texture_count: u32 = 0;
        for texture in &bindings.texture_bindings {
            match texture.desc_set {
                0 => readonly_texture_count += 1,
                1 => readwrite_texture_count += 1,
                _ => {}
            }
        }

        let mut uniform_buffer_count: u32 = 0;
        let mut readonly_buffer_count: u32 = 0;
        for buffer in &bindings.buffer_bindings {
            match buffer.desc_set {
                0 => readonly_buffer_count += 1,
                2 => uniform_buffer_count += 1,
                _ => {}
            }
        }

        for texture in &bindings.texture_bindings {
            let mut binding = *texture;
            if binding.desc_set == 0 {
                // readonly textures
                binding.msl_texture = binding.binding;
                binding.msl_sampler = binding.binding;
            } else {
                // readwrite textures
                binding.msl_texture = readonly_texture_count + binding.binding;
                binding.msl_sampler = readonly_texture_count + binding.binding;
            }
            compiler.add_msl_resource_binding(&binding)?;
        }

        for buffer in &bindings.buffer_bindings {
            let mut binding = *buffer;
            if binding.desc_set == 0 {
                // Subtract the readonly texture count because they precede
                // readonly buffers in the descriptor set.
                binding.msl_buffer = uniform_buffer_count
                    .wrapping_add(binding.binding.wrapping_sub(readonly_texture_count));
            } else if binding.desc_set == 1 {
                // Subtract the readwrite texture count because they precede
                // readwrite buffers in the descriptor set.
                binding.msl_buffer = uniform_buffer_count
                    .wrapping_add(readonly_buffer_count)
                    .wrapping_add(binding.binding.wrapping_sub(readwrite_texture_count));
            } else {
                // Uniform buffers are alone in the descriptor set.
                binding.msl_buffer = binding.binding;
            }
            compiler.add_msl_resource_binding(&binding)?;
        }
    }

    compiler.install_options(&compiler_options)?;
    let source = compiler.compile()?;

    let cleansed_entrypoint = if backend == ffi::SPVC_BACKEND_MSL {
        // Metal doesn't allow a "main" entrypoint, determine the "cleansed"
        // entrypoint name (e.g. main -> main0 on MSL).
        compiler
            .cleansed_entry_point_name(entrypoint)
            .ok_or_else(|| {
                Error::InvalidParameter("failed to get cleansed entry point name".into())
            })?
    } else {
        entrypoint.to_owned()
    };

    Ok(TranspileOutput {
        source,
        cleansed_entrypoint,
    })
}

#[derive(Default)]
struct MslBindings {
    buffer_bindings: Vec<ffi::spvc_msl_resource_binding_2>,
    texture_bindings: Vec<ffi::spvc_msl_resource_binding_2>,
}

fn push_texture_binding(
    compiler: &Compiler,
    resource: &ffi::spvc_reflected_resource,
    execution_model: ffi::SpvExecutionModel,
    valid_sets: &[u32],
    message: &str,
    bindings: &mut MslBindings,
) -> Result<()> {
    let (descriptor_set, binding_index) = require_set_and_binding(compiler, resource)?;
    if !valid_sets.contains(&descriptor_set) {
        return Err(Error::InvalidParameter(message.to_owned()));
    }
    bindings
        .texture_bindings
        .push(ffi::spvc_msl_resource_binding_2 {
            stage: execution_model,
            desc_set: descriptor_set,
            binding: binding_index,
            count: 1,
            msl_buffer: 0,
            // Assigned after all resources are collected.
            msl_texture: 0,
            msl_sampler: 0,
        });
    Ok(())
}

fn push_buffer_binding(
    compiler: &Compiler,
    resource: &ffi::spvc_reflected_resource,
    execution_model: ffi::SpvExecutionModel,
    valid_sets: &[u32],
    message: &str,
    bindings: &mut MslBindings,
) -> Result<()> {
    let (descriptor_set, binding_index) = require_set_and_binding(compiler, resource)?;
    if !valid_sets.contains(&descriptor_set) {
        return Err(Error::InvalidParameter(message.to_owned()));
    }
    bindings
        .buffer_bindings
        .push(ffi::spvc_msl_resource_binding_2 {
            stage: execution_model,
            desc_set: descriptor_set,
            binding: binding_index,
            count: 1,
            // Assigned after all resources are collected.
            msl_buffer: 0,
            msl_texture: 0,
            msl_sampler: 0,
        });
    Ok(())
}

fn require_set_and_binding(
    compiler: &Compiler,
    resource: &ffi::spvc_reflected_resource,
) -> Result<(u32, u32)> {
    if !compiler.has_decoration(resource.id, ffi::SpvDecorationDescriptorSet)
        || !compiler.has_decoration(resource.id, ffi::SpvDecorationBinding)
    {
        return Err(Error::InvalidParameter(
            "Shader resources must have descriptor set and binding index!".into(),
        ));
    }
    let descriptor_set = compiler.get_decoration(resource.id, ffi::SpvDecorationDescriptorSet);
    let binding = compiler.get_decoration(resource.id, ffi::SpvDecorationBinding);
    Ok((descriptor_set, binding))
}

fn active_resources(compiler: &Compiler) -> Result<crate::spvc::Resources> {
    let active_variables = compiler.active_interface_variables()?;
    compiler.resources_for_active_variables(active_variables)
}

fn graphics_msl_bindings(
    compiler: &Compiler,
    execution_model: ffi::SpvExecutionModel,
) -> Result<MslBindings> {
    let resources = active_resources(compiler)?;
    let mut bindings = MslBindings::default();
    let mut num_separate_samplers = 0usize;

    // Combined texture-samplers. If the source is HLSL we might have separate
    // images and samplers instead.
    let texture_samplers = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SAMPLED_IMAGE)?;
    let mut texture_sampler_list = texture_samplers;
    if texture_sampler_list.is_empty() {
        let separate_samplers =
            resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_SAMPLERS)?;
        num_separate_samplers = separate_samplers.len();
        texture_sampler_list = separate_samplers;
    }

    for resource in texture_sampler_list {
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0, 2],
            "Descriptor set index for graphics texture-sampler must be 0 or 2!",
            &mut bindings,
        )?;
    }

    // Storage textures
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_IMAGE)? {
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0, 2],
            "Descriptor set index for graphics storage texture must be 0 or 2!",
            &mut bindings,
        )?;
    }

    // If source is HLSL, storage images might be marked as separate images.
    // We only want to iterate the images that don't have an associated sampler.
    for resource in resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_IMAGE)?
        .iter()
        .skip(num_separate_samplers)
    {
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0, 2],
            "Descriptor set index for graphics storage texture must be 0 or 2!",
            &mut bindings,
        )?;
    }

    // Uniform buffers
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_UNIFORM_BUFFER)? {
        push_buffer_binding(
            compiler,
            resource,
            execution_model,
            &[1, 3],
            "Descriptor set index for graphics uniform buffer must be 1 or 3!",
            &mut bindings,
        )?;
    }

    // Storage buffers
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_BUFFER)? {
        push_buffer_binding(
            compiler,
            resource,
            execution_model,
            &[0, 2],
            "Descriptor set index for graphics storage buffer must be 0 or 2!",
            &mut bindings,
        )?;
    }

    Ok(bindings)
}

fn compute_msl_bindings(
    compiler: &Compiler,
    execution_model: ffi::SpvExecutionModel,
) -> Result<MslBindings> {
    let resources = active_resources(compiler)?;
    let mut bindings = MslBindings::default();
    let mut num_separate_samplers = 0usize;

    // Combined texture-samplers. If the source is HLSL we might have separate
    // images and samplers instead.
    let texture_samplers = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SAMPLED_IMAGE)?;
    let mut texture_sampler_list = texture_samplers;
    if texture_sampler_list.is_empty() {
        let separate_samplers =
            resources.resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_SAMPLERS)?;
        num_separate_samplers = separate_samplers.len();
        texture_sampler_list = separate_samplers;
    }

    for resource in texture_sampler_list {
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0],
            "Descriptor set index for compute texture-sampler must be 0!",
            &mut bindings,
        )?;
    }

    // Readonly storage textures
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_IMAGE)? {
        let (descriptor_set, _) = require_set_and_binding(compiler, resource)?;
        if descriptor_set != 0 && descriptor_set != 1 {
            return Err(Error::InvalidParameter(
                "Descriptor set index for compute storage texture must be 0 or 1!".into(),
            ));
        }
        // Skip readwrite textures.
        if descriptor_set != 0 {
            continue;
        }
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0],
            "Descriptor set index for compute storage texture must be 0 or 1!",
            &mut bindings,
        )?;
    }

    // If source is HLSL, readonly storage images might be marked as separate
    // images. We only want to iterate the images that don't have an associated
    // sampler.
    for resource in resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_IMAGE)?
        .iter()
        .skip(num_separate_samplers)
    {
        let (descriptor_set, _) = require_set_and_binding(compiler, resource)?;
        if descriptor_set != 0 && descriptor_set != 1 {
            return Err(Error::InvalidParameter(
                "Descriptor set index for compute storage texture must be 0 or 1!".into(),
            ));
        }
        // Skip readwrite textures.
        if descriptor_set != 0 {
            continue;
        }
        push_texture_binding(
            compiler,
            resource,
            execution_model,
            &[0],
            "Descriptor set index for compute storage texture must be 0 or 1!",
            &mut bindings,
        )?;
    }

    // Readwrite storage textures
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_IMAGE)? {
        let descriptor_set = compiler.get_decoration(resource.id, ffi::SpvDecorationDescriptorSet);
        // Skip readonly textures.
        if descriptor_set != 1 {
            continue;
        }
        let binding_index = compiler.get_decoration(resource.id, ffi::SpvDecorationBinding);
        bindings
            .texture_bindings
            .push(ffi::spvc_msl_resource_binding_2 {
                stage: execution_model,
                desc_set: descriptor_set,
                binding: binding_index,
                count: 1,
                msl_buffer: 0,
                msl_texture: 0,
                msl_sampler: 0,
            });
    }

    // If source is HLSL, readwrite storage images might be marked as separate
    // images.
    for resource in resources
        .resource_list(ffi::SPVC_RESOURCE_TYPE_SEPARATE_IMAGE)?
        .iter()
        .skip(num_separate_samplers)
    {
        let descriptor_set = compiler.get_decoration(resource.id, ffi::SpvDecorationDescriptorSet);
        // Skip readonly textures.
        if descriptor_set != 1 {
            continue;
        }
        let binding_index = compiler.get_decoration(resource.id, ffi::SpvDecorationBinding);
        bindings
            .texture_bindings
            .push(ffi::spvc_msl_resource_binding_2 {
                stage: execution_model,
                desc_set: descriptor_set,
                binding: binding_index,
                count: 1,
                msl_buffer: 0,
                msl_texture: 0,
                msl_sampler: 0,
            });
    }

    // Uniform buffers
    for resource in resources.resource_list(ffi::SPVC_RESOURCE_TYPE_UNIFORM_BUFFER)? {
        push_buffer_binding(
            compiler,
            resource,
            execution_model,
            &[2],
            "Descriptor set index for compute uniform buffer must be 2!",
            &mut bindings,
        )?;
    }

    // Storage buffers; collect readonly first, then readwrite.
    let storage_buffers = resources.resource_list(ffi::SPVC_RESOURCE_TYPE_STORAGE_BUFFER)?;
    for resource in storage_buffers {
        let (descriptor_set, _) = require_set_and_binding(compiler, resource)?;
        if descriptor_set != 0 && descriptor_set != 1 {
            return Err(Error::InvalidParameter(
                "Descriptor set index for compute storage buffer must be 0 or 1!".into(),
            ));
        }
        // Skip readwrite buffers.
        if descriptor_set != 0 {
            continue;
        }
        push_buffer_binding(
            compiler,
            resource,
            execution_model,
            &[0],
            "Descriptor set index for compute storage buffer must be 0 or 1!",
            &mut bindings,
        )?;
    }
    for resource in storage_buffers {
        let descriptor_set = compiler.get_decoration(resource.id, ffi::SpvDecorationDescriptorSet);
        // Skip readonly buffers.
        if descriptor_set != 1 {
            continue;
        }
        let binding_index = compiler.get_decoration(resource.id, ffi::SpvDecorationBinding);
        bindings
            .buffer_bindings
            .push(ffi::spvc_msl_resource_binding_2 {
                stage: execution_model,
                desc_set: descriptor_set,
                binding: binding_index,
                count: 1,
                msl_buffer: 0,
                msl_texture: 0,
                msl_sampler: 0,
            });
    }

    Ok(bindings)
}
