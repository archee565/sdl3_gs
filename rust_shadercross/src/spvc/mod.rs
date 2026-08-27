//! A minimal safe wrapper around the spirv-cross C API.
//!
//! A `Context` owns all spirv-cross allocations (via `spvc_context_destroy` on
//! drop). The other handle types (`Compiler`, `Options`, `Resources`, `Set`,
//! `Type`) are non-owning views into the context, mirroring the C API.

pub(crate) mod ffi;

use crate::error::{Error, Result};
use std::ffi::{c_char, CStr};
use std::ptr;

pub(crate) struct Context {
    raw: ffi::spvc_context,
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ffi::spvc_context_destroy(self.raw) };
    }
}

impl Context {
    pub fn new() -> Result<Self> {
        let mut raw: ffi::spvc_context = ptr::null_mut();
        let result = unsafe { ffi::spvc_context_create(&mut raw) };
        if result < ffi::SPVC_SUCCESS {
            return Err(Error::Spvc {
                function: "spvc_context_create",
                message: format!("{result:X}"),
            });
        }
        Ok(Context { raw })
    }

    fn last_error(&self) -> String {
        unsafe {
            let ptr = ffi::spvc_context_get_last_error_string(self.raw);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub(crate) fn error(&self, function: &'static str) -> Error {
        Error::Spvc {
            function,
            message: self.last_error(),
        }
    }

    pub fn parse_spirv(&self, code: &[u8]) -> Result<ParsedIr> {
        // SPIR-V is a stream of little-endian words; rebuild the words so the
        // input does not have to be 4-byte aligned.
        let words: Vec<u32> = code
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let mut ir: ffi::spvc_parsed_ir = ptr::null_mut();
        let result = unsafe {
            ffi::spvc_context_parse_spirv(self.raw, words.as_ptr(), words.len(), &mut ir)
        };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_context_parse_spirv"));
        }
        Ok(ParsedIr(ir))
    }

    pub fn create_compiler(&self, backend: ffi::spvc_backend, ir: ParsedIr) -> Result<Compiler> {
        let mut compiler: ffi::spvc_compiler = ptr::null_mut();
        let result = unsafe {
            ffi::spvc_context_create_compiler(
                self.raw,
                backend,
                ir.0,
                ffi::SPVC_CAPTURE_MODE_TAKE_OWNERSHIP,
                &mut compiler,
            )
        };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_context_create_compiler"));
        }
        Ok(Compiler {
            raw: compiler,
            context: self.raw,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ParsedIr(ffi::spvc_parsed_ir);

#[derive(Clone, Copy)]
pub(crate) struct Compiler {
    raw: ffi::spvc_compiler,
    context: ffi::spvc_context,
}

impl Compiler {
    fn context(&self) -> ContextView<'_> {
        ContextView {
            raw: self.context,
            _marker: std::marker::PhantomData,
        }
    }

    fn error(&self, function: &'static str) -> Error {
        self.context().error(function)
    }

    pub fn create_options(&self) -> Result<Options> {
        let mut options: ffi::spvc_compiler_options = ptr::null_mut();
        let result = unsafe { ffi::spvc_compiler_create_compiler_options(self.raw, &mut options) };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_create_compiler_options"));
        }
        Ok(Options { raw: options })
    }

    pub fn install_options(&self, options: &Options) -> Result<()> {
        let result = unsafe { ffi::spvc_compiler_install_compiler_options(self.raw, options.raw) };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_install_compiler_options"));
        }
        Ok(())
    }

    /// Compiles the IR; the returned string is copied out of the context.
    pub fn compile(&self) -> Result<String> {
        let mut source: *const c_char = ptr::null();
        let result = unsafe { ffi::spvc_compiler_compile(self.raw, &mut source) };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_compile"));
        }
        let string = unsafe { CStr::from_ptr(source) }
            .to_string_lossy()
            .into_owned();
        Ok(string)
    }

    #[allow(dead_code)]
    pub fn execution_model(&self) -> ffi::SpvExecutionModel {
        unsafe { ffi::spvc_compiler_get_execution_model(self.raw) }
    }

    /// Metal does not allow a `main` entrypoint; ask spirv-cross for the
    /// "cleansed" name (e.g. `main` becomes `main0`).
    pub fn cleansed_entry_point_name(&self, entrypoint: &str) -> Option<String> {
        let entrypoint = match std::ffi::CString::new(entrypoint) {
            Ok(s) => s,
            Err(_) => return None,
        };
        unsafe {
            let ptr = ffi::spvc_compiler_get_cleansed_entry_point_name(
                self.raw,
                entrypoint.as_ptr(),
                ffi::spvc_compiler_get_execution_model(self.raw),
            );
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    }

    pub fn active_interface_variables(&self) -> Result<Set> {
        let mut set: ffi::spvc_set = ptr::null();
        let result =
            unsafe { ffi::spvc_compiler_get_active_interface_variables(self.raw, &mut set) };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_get_active_interface_variables"));
        }
        Ok(Set(set))
    }

    pub fn resources_for_active_variables(&self, active: Set) -> Result<Resources> {
        let mut resources: ffi::spvc_resources = ptr::null_mut();
        let result = unsafe {
            ffi::spvc_compiler_create_shader_resources_for_active_variables(
                self.raw,
                &mut resources,
                active.0,
            )
        };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_create_shader_resources_for_active_variables"));
        }
        Ok(Resources {
            raw: resources,
            context: self.context,
        })
    }

    pub fn has_decoration(&self, id: ffi::SpvId, decoration: ffi::SpvDecoration) -> bool {
        unsafe { ffi::spvc_compiler_has_decoration(self.raw, id, decoration) != 0 }
    }

    pub fn get_decoration(&self, id: ffi::SpvId, decoration: ffi::SpvDecoration) -> u32 {
        unsafe { ffi::spvc_compiler_get_decoration(self.raw, id, decoration) }
    }

    pub fn add_msl_resource_binding(
        &self,
        binding: &ffi::spvc_msl_resource_binding_2,
    ) -> Result<()> {
        let result = unsafe { ffi::spvc_compiler_msl_add_resource_binding_2(self.raw, binding) };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_compiler_msl_add_resource_binding_2"));
        }
        Ok(())
    }

    pub fn type_handle(&self, id: ffi::spvc_type_id) -> Type {
        Type(unsafe { ffi::spvc_compiler_get_type_handle(self.raw, id) })
    }

    pub fn execution_mode_argument(&self, mode: ffi::SpvExecutionMode, index: u32) -> u32 {
        unsafe { ffi::spvc_compiler_get_execution_mode_argument_by_index(self.raw, mode, index) }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Options {
    raw: ffi::spvc_compiler_options,
}

impl Options {
    pub fn set_bool(&self, option: ffi::spvc_compiler_option, value: bool) -> Result<()> {
        let result =
            unsafe { ffi::spvc_compiler_options_set_bool(self.raw, option, u8::from(value)) };
        if result < ffi::SPVC_SUCCESS {
            // Without a compiler handle we cannot fetch the context error string.
            return Err(Error::Spvc {
                function: "spvc_compiler_options_set_bool",
                message: format!("{result:X}"),
            });
        }
        Ok(())
    }

    pub fn set_uint(&self, option: ffi::spvc_compiler_option, value: u32) -> Result<()> {
        let result = unsafe { ffi::spvc_compiler_options_set_uint(self.raw, option, value) };
        if result < ffi::SPVC_SUCCESS {
            return Err(Error::Spvc {
                function: "spvc_compiler_options_set_uint",
                message: format!("{result:X}"),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Resources {
    raw: ffi::spvc_resources,
    context: ffi::spvc_context,
}

impl Resources {
    fn error(&self, function: &'static str) -> Error {
        ContextView {
            raw: self.context,
            _marker: std::marker::PhantomData,
        }
        .error(function)
    }

    /// Returns the list of reflected resources of the given type. The slice
    /// borrows from the context, which must stay alive.
    pub fn resource_list(
        &self,
        resource_type: ffi::spvc_resource_type,
    ) -> Result<&[ffi::spvc_reflected_resource]> {
        let mut list: *const ffi::spvc_reflected_resource = ptr::null();
        let mut count: usize = 0;
        let result = unsafe {
            ffi::spvc_resources_get_resource_list_for_type(
                self.raw,
                resource_type,
                &mut list,
                &mut count,
            )
        };
        if result < ffi::SPVC_SUCCESS {
            return Err(self.error("spvc_resources_get_resource_list_for_type"));
        }
        if list.is_null() || count == 0 {
            return Ok(&[]);
        }
        Ok(unsafe { std::slice::from_raw_parts(list, count) })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Set(ffi::spvc_set);

#[derive(Clone, Copy)]
pub(crate) struct Type(ffi::spvc_type);

impl Type {
    pub fn basetype(&self) -> ffi::spvc_basetype {
        unsafe { ffi::spvc_type_get_basetype(self.0) }
    }

    pub fn vector_size(&self) -> u32 {
        unsafe { ffi::spvc_type_get_vector_size(self.0) }
    }
}

/// Temporary access to the context for error string retrieval. The caller
/// guarantees the context outlives the view.
struct ContextView<'a> {
    raw: ffi::spvc_context,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl ContextView<'_> {
    fn error(&self, function: &'static str) -> Error {
        Error::Spvc {
            function,
            message: unsafe {
                let ptr = ffi::spvc_context_get_last_error_string(self.raw);
                if ptr.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            },
        }
    }
}
