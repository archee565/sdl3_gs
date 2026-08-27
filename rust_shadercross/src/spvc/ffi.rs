//! Hand-written FFI declarations for the subset of the spirv-cross C API
//! (spirv_cross_c.h, shipped with the Vulkan SDK as libspirv-cross-c-shared)
//! that SDL_shadercross uses.
//!
//! Enum values mirror /usr/include/spirv_cross/spirv_cross_c.h and the
//! SPIR-V core grammar (spirv.h); do not change them lightly.

#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]

use std::os::raw::{c_char, c_uchar, c_uint};

#[link(name = "spirv-cross-c-shared")]
extern "C" {
    pub fn spvc_context_create(context: *mut spvc_context) -> spvc_result;
    pub fn spvc_context_destroy(context: spvc_context);
    pub fn spvc_context_get_last_error_string(context: spvc_context) -> *const c_char;
    pub fn spvc_context_parse_spirv(
        context: spvc_context,
        spirv: *const SpvId,
        word_count: usize,
        parsed_ir: *mut spvc_parsed_ir,
    ) -> spvc_result;
    pub fn spvc_context_create_compiler(
        context: spvc_context,
        backend: spvc_backend,
        parsed_ir: spvc_parsed_ir,
        mode: spvc_capture_mode,
        compiler: *mut spvc_compiler,
    ) -> spvc_result;

    pub fn spvc_compiler_create_compiler_options(
        compiler: spvc_compiler,
        options: *mut spvc_compiler_options,
    ) -> spvc_result;
    pub fn spvc_compiler_options_set_bool(
        options: spvc_compiler_options,
        option: spvc_compiler_option,
        value: spvc_bool,
    ) -> spvc_result;
    pub fn spvc_compiler_options_set_uint(
        options: spvc_compiler_options,
        option: spvc_compiler_option,
        value: c_uint,
    ) -> spvc_result;
    pub fn spvc_compiler_install_compiler_options(
        compiler: spvc_compiler,
        options: spvc_compiler_options,
    ) -> spvc_result;
    pub fn spvc_compiler_compile(
        compiler: spvc_compiler,
        source: *mut *const c_char,
    ) -> spvc_result;
    pub fn spvc_compiler_get_execution_model(compiler: spvc_compiler) -> SpvExecutionModel;
    pub fn spvc_compiler_get_cleansed_entry_point_name(
        compiler: spvc_compiler,
        name: *const c_char,
        execution_model: SpvExecutionModel,
    ) -> *const c_char;
    pub fn spvc_compiler_get_active_interface_variables(
        compiler: spvc_compiler,
        set: *mut spvc_set,
    ) -> spvc_result;
    pub fn spvc_compiler_create_shader_resources_for_active_variables(
        compiler: spvc_compiler,
        resources: *mut spvc_resources,
        active: spvc_set,
    ) -> spvc_result;
    pub fn spvc_resources_get_resource_list_for_type(
        resources: spvc_resources,
        resource_type: spvc_resource_type,
        resource_list: *mut *const spvc_reflected_resource,
        resource_size: *mut usize,
    ) -> spvc_result;
    pub fn spvc_compiler_has_decoration(
        compiler: spvc_compiler,
        id: SpvId,
        decoration: SpvDecoration,
    ) -> spvc_bool;
    pub fn spvc_compiler_get_decoration(
        compiler: spvc_compiler,
        id: SpvId,
        decoration: SpvDecoration,
    ) -> c_uint;
    pub fn spvc_compiler_msl_add_resource_binding_2(
        compiler: spvc_compiler,
        binding: *const spvc_msl_resource_binding_2,
    ) -> spvc_result;
    pub fn spvc_compiler_get_type_handle(compiler: spvc_compiler, id: spvc_type_id) -> spvc_type;
    pub fn spvc_type_get_basetype(spirv_type: spvc_type) -> spvc_basetype;
    pub fn spvc_type_get_vector_size(spirv_type: spvc_type) -> c_uint;
    pub fn spvc_compiler_get_execution_mode_argument_by_index(
        compiler: spvc_compiler,
        execution_mode: SpvExecutionMode,
        index: c_uint,
    ) -> c_uint;
}

/* Handles. All child objects are owned by the context. */
pub type spvc_context = *mut spvc_context_s;
pub type spvc_parsed_ir = *mut spvc_parsed_ir_s;
pub type spvc_compiler = *mut spvc_compiler_s;
pub type spvc_compiler_options = *mut spvc_compiler_options_s;
pub type spvc_resources = *mut spvc_resources_s;
pub type spvc_type = *const spvc_type_s;
pub type spvc_set = *const spvc_set_s;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_context_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_parsed_ir_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_compiler_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_compiler_options_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_resources_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_type_s {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_set_s {
    _unused: [u8; 0],
}

pub type SpvId = c_uint;
pub type spvc_type_id = SpvId;
pub type spvc_variable_id = SpvId;
pub type spvc_bool = c_uchar;

pub type spvc_result = i32;
pub const SPVC_SUCCESS: spvc_result = 0;

pub type spvc_capture_mode = i32;
pub const SPVC_CAPTURE_MODE_TAKE_OWNERSHIP: spvc_capture_mode = 1;

pub type spvc_backend = i32;
pub const SPVC_BACKEND_NONE: spvc_backend = 0;
pub const SPVC_BACKEND_HLSL: spvc_backend = 2;
pub const SPVC_BACKEND_MSL: spvc_backend = 3;

pub type spvc_resource_type = i32;
pub const SPVC_RESOURCE_TYPE_UNIFORM_BUFFER: spvc_resource_type = 1;
pub const SPVC_RESOURCE_TYPE_STORAGE_BUFFER: spvc_resource_type = 2;
pub const SPVC_RESOURCE_TYPE_STAGE_INPUT: spvc_resource_type = 3;
pub const SPVC_RESOURCE_TYPE_STAGE_OUTPUT: spvc_resource_type = 4;
pub const SPVC_RESOURCE_TYPE_STORAGE_IMAGE: spvc_resource_type = 6;
pub const SPVC_RESOURCE_TYPE_SAMPLED_IMAGE: spvc_resource_type = 7;
pub const SPVC_RESOURCE_TYPE_SEPARATE_IMAGE: spvc_resource_type = 10;
pub const SPVC_RESOURCE_TYPE_SEPARATE_SAMPLERS: spvc_resource_type = 11;

pub type spvc_basetype = i32;
pub const SPVC_BASETYPE_UNKNOWN: spvc_basetype = 0;
pub const SPVC_BASETYPE_INT8: spvc_basetype = 3;
pub const SPVC_BASETYPE_UINT8: spvc_basetype = 4;
pub const SPVC_BASETYPE_INT16: spvc_basetype = 5;
pub const SPVC_BASETYPE_UINT16: spvc_basetype = 6;
pub const SPVC_BASETYPE_INT32: spvc_basetype = 7;
pub const SPVC_BASETYPE_UINT32: spvc_basetype = 8;
pub const SPVC_BASETYPE_INT64: spvc_basetype = 9;
pub const SPVC_BASETYPE_UINT64: spvc_basetype = 10;
pub const SPVC_BASETYPE_FP16: spvc_basetype = 12;
pub const SPVC_BASETYPE_FP32: spvc_basetype = 13;
pub const SPVC_BASETYPE_FP64: spvc_basetype = 14;

/* spvc_compiler_option values, see spirv_cross_c.h. */
pub type spvc_compiler_option = u32;
pub const SPVC_COMPILER_OPTION_COMMON_BIT: u32 = 0x100_0000;
pub const SPVC_COMPILER_OPTION_HLSL_BIT: u32 = 0x400_0000;
pub const SPVC_COMPILER_OPTION_MSL_BIT: u32 = 0x800_0000;
pub const SPVC_COMPILER_OPTION_HLSL_SHADER_MODEL: spvc_compiler_option =
    13 | SPVC_COMPILER_OPTION_HLSL_BIT;
pub const SPVC_COMPILER_OPTION_HLSL_POINT_SIZE_COMPAT: spvc_compiler_option =
    14 | SPVC_COMPILER_OPTION_HLSL_BIT;
pub const SPVC_COMPILER_OPTION_MSL_VERSION: spvc_compiler_option =
    17 | SPVC_COMPILER_OPTION_MSL_BIT;
pub const SPVC_COMPILER_OPTION_HLSL_NONWRITABLE_UAV_TEXTURE_AS_SRV: spvc_compiler_option =
    55 | SPVC_COMPILER_OPTION_HLSL_BIT;
pub const SPVC_COMPILER_OPTION_HLSL_FLATTEN_MATRIX_VERTEX_INPUT_SEMANTICS: spvc_compiler_option =
    71 | SPVC_COMPILER_OPTION_HLSL_BIT;
pub const SPVC_COMPILER_OPTION_HLSL_USE_ENTRY_POINT_NAME: spvc_compiler_option =
    90 | SPVC_COMPILER_OPTION_HLSL_BIT;

/* SPIR-V core enums (spirv.h from SPIRV-Headers). */
pub type SpvExecutionModel = c_uint;
pub const SpvExecutionModelVertex: SpvExecutionModel = 0;
pub const SpvExecutionModelFragment: SpvExecutionModel = 4;
pub const SpvExecutionModelGLCompute: SpvExecutionModel = 5;
pub const SpvExecutionModelKernel: SpvExecutionModel = 6;

pub type SpvDecoration = c_uint;
pub const SpvDecorationLocation: SpvDecoration = 30;
pub const SpvDecorationBinding: SpvDecoration = 33;
pub const SpvDecorationDescriptorSet: SpvDecoration = 34;

pub type SpvExecutionMode = c_uint;
pub const SpvExecutionModeLocalSize: SpvExecutionMode = 17;

/* Maps to C++ API. */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_reflected_resource {
    pub id: spvc_variable_id,
    pub base_type_id: spvc_type_id,
    pub type_id: spvc_type_id,
    pub name: *const c_char,
}

/* Maps to C++ API. */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct spvc_msl_resource_binding_2 {
    pub stage: SpvExecutionModel,
    pub desc_set: c_uint,
    pub binding: c_uint,
    pub count: c_uint,
    pub msl_buffer: c_uint,
    pub msl_texture: c_uint,
    pub msl_sampler: c_uint,
}
