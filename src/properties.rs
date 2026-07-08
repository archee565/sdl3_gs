use sdl3_sys as sys;
use std::ffi::{CStr, CString};

pub use sys::properties::{SDL_PropertiesID, SDL_PropertyType};

/// Safe wrapper around `SDL_PropertiesID`.
///
/// When created via [`Properties::new()`], the underlying `SDL_PropertiesID` is
/// destroyed on `Drop`. Wrappers obtained via [`Properties::from_raw()`] are **not**
/// destroyed on drop (the caller is responsible, or the ID is managed externally by SDL).
pub struct Properties {
    id: SDL_PropertiesID,
    owned: bool,
}

impl Properties {
    /// Create a new property group. The group is destroyed on drop.
    pub fn new() -> Result<Self, &'static str> {
        let id = unsafe { sys::properties::SDL_CreateProperties() };
        if id == SDL_PropertiesID(0) {
            Err(get_error())
        } else {
            Ok(Self { id, owned: true })
        }
    }

    /// Wrap an existing `SDL_PropertiesID` **without** taking ownership.
    /// The ID will NOT be destroyed on drop.
    pub fn from_raw(id: SDL_PropertiesID) -> Self {
        Self { id, owned: false }
    }

    /// Returns the global SDL properties. Does not take ownership.
    pub fn global() -> Result<Self, &'static str> {
        let id = unsafe { sys::properties::SDL_GetGlobalProperties() };
        if id == SDL_PropertiesID(0) {
            Err(get_error())
        } else {
            Ok(Self::from_raw(id))
        }
    }

    pub fn raw(&self) -> SDL_PropertiesID {
        self.id
    }

    /// Copy all properties from `src` into this property group.
    pub fn copy_from(&self, src: &Properties) -> Result<(), &'static str> {
        let ok = unsafe { sys::properties::SDL_CopyProperties(src.id, self.id) };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    /// Lock the property group for multi-threaded access.
    pub fn lock(&self) -> Result<(), &'static str> {
        let ok = unsafe { sys::properties::SDL_LockProperties(self.id) };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    /// Unlock the property group.
    pub fn unlock(&self) {
        unsafe { sys::properties::SDL_UnlockProperties(self.id) }
    }

    /// Returns `true` if the named property exists.
    pub fn has(&self, name: &str) -> bool {
        let c_name = CString::new(name).unwrap();
        unsafe { sys::properties::SDL_HasProperty(self.id, c_name.as_ptr()) }
    }

    /// Returns the type of the named property, or `None` if not set.
    pub fn get_type(&self, name: &str) -> Option<PropertyType> {
        let c_name = CString::new(name).unwrap();
        let t = unsafe { sys::properties::SDL_GetPropertyType(self.id, c_name.as_ptr()) };
        PropertyType::from_raw(t)
    }

    // -- String --

    pub fn set_string(&self, name: &str, value: &str) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let c_value = CString::new(value).unwrap();
        let ok = unsafe {
            sys::properties::SDL_SetStringProperty(self.id, c_name.as_ptr(), c_value.as_ptr())
        };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    pub fn get_string(&self, name: &str) -> Option<&str> {
        let c_name = CString::new(name).unwrap();
        let ptr = unsafe {
            sys::properties::SDL_GetStringProperty(self.id, c_name.as_ptr(), std::ptr::null())
        };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or(""))
    }

    // -- Number (i64) --

    pub fn set_number(&self, name: &str, value: i64) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let ok = unsafe {
            sys::properties::SDL_SetNumberProperty(self.id, c_name.as_ptr(), value)
        };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    pub fn get_number(&self, name: &str, default: i64) -> i64 {
        let c_name = CString::new(name).unwrap();
        unsafe { sys::properties::SDL_GetNumberProperty(self.id, c_name.as_ptr(), default) }
    }

    // -- Float (f32) --

    pub fn set_float(&self, name: &str, value: f32) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let ok = unsafe {
            sys::properties::SDL_SetFloatProperty(self.id, c_name.as_ptr(), value)
        };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    pub fn get_float(&self, name: &str, default: f32) -> f32 {
        let c_name = CString::new(name).unwrap();
        unsafe { sys::properties::SDL_GetFloatProperty(self.id, c_name.as_ptr(), default) }
    }

    // -- Boolean --

    pub fn set_bool(&self, name: &str, value: bool) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let ok = unsafe {
            sys::properties::SDL_SetBooleanProperty(self.id, c_name.as_ptr(), value)
        };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    pub fn get_bool(&self, name: &str, default: bool) -> bool {
        let c_name = CString::new(name).unwrap();
        unsafe { sys::properties::SDL_GetBooleanProperty(self.id, c_name.as_ptr(), default) }
    }

    // -- Data --

    pub fn set_data(&self, name: &str, value: &[u8]) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let boxed = Box::new(value.to_vec());
        let ptr = Box::into_raw(boxed) as *mut std::ffi::c_void;
        let ok = unsafe {
            sys::properties::SDL_SetPointerPropertyWithCleanup(
                self.id,
                c_name.as_ptr(),
                ptr,
                Some(cleanup_vec),
                std::ptr::null_mut(),
            )
        };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    pub fn get_data(&self, name: &str) -> Option<&[u8]> {
        let c_name = CString::new(name).unwrap();
        let ptr = unsafe {
            sys::properties::SDL_GetPointerProperty(self.id, c_name.as_ptr(), std::ptr::null_mut())
        };
        if ptr.is_null() {
            None
        } else {
            let vec = unsafe { &*(ptr as *const Vec<u8>) };
            Some(vec.as_slice())
        }
    }

    pub fn get_pointer(&self, name: &str) -> *mut std::ffi::c_void {
        let c_name = CString::new(name).unwrap();
        unsafe {
            sys::properties::SDL_GetPointerProperty(self.id, c_name.as_ptr(), std::ptr::null_mut())
        }
    }

    // -- Clear --

    pub fn clear(&self, name: &str) -> Result<(), &'static str> {
        let c_name = CString::new(name).unwrap();
        let ok = unsafe { sys::properties::SDL_ClearProperty(self.id, c_name.as_ptr()) };
        if ok { Ok(()) } else { Err(get_error()) }
    }

    // -- Enumerate --

    /// Enumerate all property names in this group. Calls `f` once per property name.
    pub fn enumerate<F>(&self, mut f: F)
    where
        F: FnMut(&str),
    {
        let ctx = &mut f as *mut F as *mut std::ffi::c_void;
        unsafe {
            sys::properties::SDL_EnumerateProperties(
                self.id,
                Some(enumerate_callback::<F>),
                ctx,
            );
        }
    }

    /// Collect all property names into a `Vec<String>`.
    pub fn property_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        self.enumerate(|name| names.push(name.to_string()));
        names
    }
}

unsafe extern "C" fn enumerate_callback<F: FnMut(&str)>(
    _userdata: *mut std::ffi::c_void,
    _props: SDL_PropertiesID,
    name: *const std::ffi::c_char,
) {
    let s = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");
    let f = unsafe { &mut *(_userdata as *mut F) };
    f(s);
}

impl Drop for Properties {
    fn drop(&mut self) {
        if self.owned && self.id != SDL_PropertiesID(0) {
            unsafe { sys::properties::SDL_DestroyProperties(self.id) }
        }
    }
}

/// Safe property type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    Pointer,
    String,
    Number,
    Float,
    Boolean,
}

impl PropertyType {
    fn from_raw(t: SDL_PropertyType) -> Option<Self> {
        match t {
            sys::properties::SDL_PropertyType::POINTER => Some(Self::Pointer),
            sys::properties::SDL_PropertyType::STRING => Some(Self::String),
            sys::properties::SDL_PropertyType::NUMBER => Some(Self::Number),
            sys::properties::SDL_PropertyType::FLOAT => Some(Self::Float),
            sys::properties::SDL_PropertyType::BOOLEAN => Some(Self::Boolean),
            _ => None,
        }
    }
}

fn get_error() -> &'static str {
    let ptr = sys::error::SDL_GetError();
    if ptr.is_null() {
        return "unknown error";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("unknown error")
}

unsafe extern "C" fn cleanup_vec(_userdata: *mut std::ffi::c_void, value: *mut std::ffi::c_void) {
    let _ = unsafe { Box::from_raw(value as *mut Vec<u8>) };
}

// ---------------------------------------------------------------------------
// Property name constants
// ---------------------------------------------------------------------------
//
// These are safe `&str` equivalents of the `*const c_char` constants in
// `sdl3_sys`. C string → &str conversion cannot happen at compile time, so
// these are the literal string values. Gamepad aliases reference the joystick
// constants (matching the aliasing in sdl3-sys).

// -- Properties --
pub const PROP_NAME_STRING: &str = "SDL.name";

// -- App metadata --
pub const PROP_APP_METADATA_NAME_STRING: &str = "SDL.app.metadata.name";
pub const PROP_APP_METADATA_VERSION_STRING: &str = "SDL.app.metadata.version";
pub const PROP_APP_METADATA_IDENTIFIER_STRING: &str = "SDL.app.metadata.identifier";
pub const PROP_APP_METADATA_CREATOR_STRING: &str = "SDL.app.metadata.creator";
pub const PROP_APP_METADATA_COPYRIGHT_STRING: &str = "SDL.app.metadata.copyright";
pub const PROP_APP_METADATA_URL_STRING: &str = "SDL.app.metadata.url";
pub const PROP_APP_METADATA_TYPE_STRING: &str = "SDL.app.metadata.type";

// -- Audio --
pub const PROP_AUDIOSTREAM_AUTO_CLEANUP_BOOLEAN: &str = "SDL.audiostream.auto_cleanup";

// -- Dialog --
pub const PROP_FILE_DIALOG_FILTERS_POINTER: &str = "SDL.filedialog.filters";
pub const PROP_FILE_DIALOG_NFILTERS_NUMBER: &str = "SDL.filedialog.nfilters";
pub const PROP_FILE_DIALOG_WINDOW_POINTER: &str = "SDL.filedialog.window";
pub const PROP_FILE_DIALOG_LOCATION_STRING: &str = "SDL.filedialog.location";
pub const PROP_FILE_DIALOG_MANY_BOOLEAN: &str = "SDL.filedialog.many";
pub const PROP_FILE_DIALOG_TITLE_STRING: &str = "SDL.filedialog.title";
pub const PROP_FILE_DIALOG_ACCEPT_STRING: &str = "SDL.filedialog.accept";
pub const PROP_FILE_DIALOG_CANCEL_STRING: &str = "SDL.filedialog.cancel";

// -- Display --
pub const PROP_DISPLAY_HDR_ENABLED_BOOLEAN: &str = "SDL.display.HDR_enabled";
pub const PROP_DISPLAY_KMSDRM_PANEL_ORIENTATION_NUMBER: &str = "SDL.display.KMSDRM.panel_orientation";
pub const PROP_DISPLAY_WAYLAND_WL_OUTPUT_POINTER: &str = "SDL.display.wayland.wl_output";
pub const PROP_DISPLAY_WINDOWS_HMONITOR_POINTER: &str = "SDL.display.windows.hmonitor";

// -- GPU device creation --
pub const PROP_GPU_DEVICE_CREATE_DEBUGMODE_BOOLEAN: &str = "SDL.gpu.device.create.debugmode";
pub const PROP_GPU_DEVICE_CREATE_PREFERLOWPOWER_BOOLEAN: &str = "SDL.gpu.device.create.preferlowpower";
pub const PROP_GPU_DEVICE_CREATE_VERBOSE_BOOLEAN: &str = "SDL.gpu.device.create.verbose";
pub const PROP_GPU_DEVICE_CREATE_NAME_STRING: &str = "SDL.gpu.device.create.name";
pub const PROP_GPU_DEVICE_CREATE_FEATURE_CLIP_DISTANCE_BOOLEAN: &str = "SDL.gpu.device.create.feature.clip_distance";
pub const PROP_GPU_DEVICE_CREATE_FEATURE_DEPTH_CLAMPING_BOOLEAN: &str = "SDL.gpu.device.create.feature.depth_clamping";
pub const PROP_GPU_DEVICE_CREATE_FEATURE_INDIRECT_DRAW_FIRST_INSTANCE_BOOLEAN: &str = "SDL.gpu.device.create.feature.indirect_draw_first_instance";
pub const PROP_GPU_DEVICE_CREATE_FEATURE_ANISOTROPY_BOOLEAN: &str = "SDL.gpu.device.create.feature.anisotropy";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_PRIVATE_BOOLEAN: &str = "SDL.gpu.device.create.shaders.private";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_SPIRV_BOOLEAN: &str = "SDL.gpu.device.create.shaders.spirv";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_DXBC_BOOLEAN: &str = "SDL.gpu.device.create.shaders.dxbc";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_DXIL_BOOLEAN: &str = "SDL.gpu.device.create.shaders.dxil";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_MSL_BOOLEAN: &str = "SDL.gpu.device.create.shaders.msl";
pub const PROP_GPU_DEVICE_CREATE_SHADERS_METALLIB_BOOLEAN: &str = "SDL.gpu.device.create.shaders.metallib";
pub const PROP_GPU_DEVICE_CREATE_D3D12_ALLOW_FEWER_RESOURCE_SLOTS_BOOLEAN: &str = "SDL.gpu.device.create.d3d12.allowtier1resourcebinding";
pub const PROP_GPU_DEVICE_CREATE_D3D12_SEMANTIC_NAME_STRING: &str = "SDL.gpu.device.create.d3d12.semantic";
pub const PROP_GPU_DEVICE_CREATE_D3D12_AGILITY_SDK_VERSION_NUMBER: &str = "SDL.gpu.device.create.d3d12.agility_sdk_version";
pub const PROP_GPU_DEVICE_CREATE_D3D12_AGILITY_SDK_PATH_STRING: &str = "SDL.gpu.device.create.d3d12.agility_sdk_path";
pub const PROP_GPU_DEVICE_CREATE_VULKAN_REQUIRE_HARDWARE_ACCELERATION_BOOLEAN: &str = "SDL.gpu.device.create.vulkan.requirehardwareacceleration";
pub const PROP_GPU_DEVICE_CREATE_VULKAN_OPTIONS_POINTER: &str = "SDL.gpu.device.create.vulkan.options";
pub const PROP_GPU_DEVICE_CREATE_METAL_ALLOW_MACFAMILY1_BOOLEAN: &str = "SDL.gpu.device.create.metal.allowmacfamily1";

// -- GPU device info --
pub const PROP_GPU_DEVICE_NAME_STRING: &str = "SDL.gpu.device.name";
pub const PROP_GPU_DEVICE_DRIVER_NAME_STRING: &str = "SDL.gpu.device.driver_name";
pub const PROP_GPU_DEVICE_DRIVER_VERSION_STRING: &str = "SDL.gpu.device.driver_version";
pub const PROP_GPU_DEVICE_DRIVER_INFO_STRING: &str = "SDL.gpu.device.driver_info";

// -- GPU resource creation --
pub const PROP_GPU_COMPUTEPIPELINE_CREATE_NAME_STRING: &str = "SDL.gpu.computepipeline.create.name";
pub const PROP_GPU_GRAPHICSPIPELINE_CREATE_NAME_STRING: &str = "SDL.gpu.graphicspipeline.create.name";
pub const PROP_GPU_SAMPLER_CREATE_NAME_STRING: &str = "SDL.gpu.sampler.create.name";
pub const PROP_GPU_SHADER_CREATE_NAME_STRING: &str = "SDL.gpu.shader.create.name";
pub const PROP_GPU_TEXTURE_CREATE_NAME_STRING: &str = "SDL.gpu.texture.create.name";
pub const PROP_GPU_BUFFER_CREATE_NAME_STRING: &str = "SDL.gpu.buffer.create.name";
pub const PROP_GPU_TRANSFERBUFFER_CREATE_NAME_STRING: &str = "SDL.gpu.transferbuffer.create.name";

// -- GPU texture D3D12 clear values --
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_R_FLOAT: &str = "SDL.gpu.texture.create.d3d12.clear.r";
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_G_FLOAT: &str = "SDL.gpu.texture.create.d3d12.clear.g";
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_B_FLOAT: &str = "SDL.gpu.texture.create.d3d12.clear.b";
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_A_FLOAT: &str = "SDL.gpu.texture.create.d3d12.clear.a";
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_DEPTH_FLOAT: &str = "SDL.gpu.texture.create.d3d12.clear.depth";
pub const PROP_GPU_TEXTURE_CREATE_D3D12_CLEAR_STENCIL_NUMBER: &str = "SDL.gpu.texture.create.d3d12.clear.stencil";

// -- HIDAPI --
pub const PROP_HIDAPI_LIBUSB_DEVICE_HANDLE_POINTER: &str = "SDL.hidapi.libusb.device.handle";

// -- IO stream --
pub const PROP_IOSTREAM_WINDOWS_HANDLE_POINTER: &str = "SDL.iostream.windows.handle";
pub const PROP_IOSTREAM_STDIO_FILE_POINTER: &str = "SDL.iostream.stdio.file";
pub const PROP_IOSTREAM_FILE_DESCRIPTOR_NUMBER: &str = "SDL.iostream.file_descriptor";
pub const PROP_IOSTREAM_ANDROID_AASSET_POINTER: &str = "SDL.iostream.android.aasset";
pub const PROP_IOSTREAM_MEMORY_POINTER: &str = "SDL.iostream.memory.base";
pub const PROP_IOSTREAM_MEMORY_SIZE_NUMBER: &str = "SDL.iostream.memory.size";
pub const PROP_IOSTREAM_MEMORY_FREE_FUNC_POINTER: &str = "SDL.iostream.memory.free";
pub const PROP_IOSTREAM_DYNAMIC_MEMORY_POINTER: &str = "SDL.iostream.dynamic.memory";
pub const PROP_IOSTREAM_DYNAMIC_CHUNKSIZE_NUMBER: &str = "SDL.iostream.dynamic.chunksize";

// -- Joystick capabilities --
pub const PROP_JOYSTICK_CAP_MONO_LED_BOOLEAN: &str = "SDL.joystick.cap.mono_led";
pub const PROP_JOYSTICK_CAP_RGB_LED_BOOLEAN: &str = "SDL.joystick.cap.rgb_led";
pub const PROP_JOYSTICK_CAP_PLAYER_LED_BOOLEAN: &str = "SDL.joystick.cap.player_led";
pub const PROP_JOYSTICK_CAP_RUMBLE_BOOLEAN: &str = "SDL.joystick.cap.rumble";
pub const PROP_JOYSTICK_CAP_TRIGGER_RUMBLE_BOOLEAN: &str = "SDL.joystick.cap.trigger_rumble";

// -- Gamepad capabilities (aliases of joystick) --
pub const PROP_GAMEPAD_CAP_MONO_LED_BOOLEAN: &str = PROP_JOYSTICK_CAP_MONO_LED_BOOLEAN;
pub const PROP_GAMEPAD_CAP_RGB_LED_BOOLEAN: &str = PROP_JOYSTICK_CAP_RGB_LED_BOOLEAN;
pub const PROP_GAMEPAD_CAP_PLAYER_LED_BOOLEAN: &str = PROP_JOYSTICK_CAP_PLAYER_LED_BOOLEAN;
pub const PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN: &str = PROP_JOYSTICK_CAP_RUMBLE_BOOLEAN;
pub const PROP_GAMEPAD_CAP_TRIGGER_RUMBLE_BOOLEAN: &str = PROP_JOYSTICK_CAP_TRIGGER_RUMBLE_BOOLEAN;

// -- Keyboard / text input --
pub const PROP_TEXTINPUT_TYPE_NUMBER: &str = "SDL.textinput.type";
pub const PROP_TEXTINPUT_CAPITALIZATION_NUMBER: &str = "SDL.textinput.capitalization";
pub const PROP_TEXTINPUT_AUTOCORRECT_BOOLEAN: &str = "SDL.textinput.autocorrect";
pub const PROP_TEXTINPUT_MULTILINE_BOOLEAN: &str = "SDL.textinput.multiline";
pub const PROP_TEXTINPUT_ANDROID_INPUTTYPE_NUMBER: &str = "SDL.textinput.android.inputtype";

// -- Process --
pub const PROP_PROCESS_CREATE_ARGS_POINTER: &str = "SDL.process.create.args";
pub const PROP_PROCESS_CREATE_ENVIRONMENT_POINTER: &str = "SDL.process.create.environment";
pub const PROP_PROCESS_CREATE_WORKING_DIRECTORY_STRING: &str = "SDL.process.create.working_directory";
pub const PROP_PROCESS_CREATE_STDIN_NUMBER: &str = "SDL.process.create.stdin_option";
pub const PROP_PROCESS_CREATE_STDIN_POINTER: &str = "SDL.process.create.stdin_source";
pub const PROP_PROCESS_CREATE_STDOUT_NUMBER: &str = "SDL.process.create.stdout_option";
pub const PROP_PROCESS_CREATE_STDOUT_POINTER: &str = "SDL.process.create.stdout_source";
pub const PROP_PROCESS_CREATE_STDERR_NUMBER: &str = "SDL.process.create.stderr_option";
pub const PROP_PROCESS_CREATE_STDERR_POINTER: &str = "SDL.process.create.stderr_source";
pub const PROP_PROCESS_CREATE_STDERR_TO_STDOUT_BOOLEAN: &str = "SDL.process.create.stderr_to_stdout";
pub const PROP_PROCESS_CREATE_BACKGROUND_BOOLEAN: &str = "SDL.process.create.background";
pub const PROP_PROCESS_CREATE_CMDLINE_STRING: &str = "SDL.process.create.cmdline";
pub const PROP_PROCESS_PID_NUMBER: &str = "SDL.process.pid";
pub const PROP_PROCESS_STDIN_POINTER: &str = "SDL.process.stdin";
pub const PROP_PROCESS_STDOUT_POINTER: &str = "SDL.process.stdout";
pub const PROP_PROCESS_STDERR_POINTER: &str = "SDL.process.stderr";
pub const PROP_PROCESS_BACKGROUND_BOOLEAN: &str = "SDL.process.background";

// -- Renderer creation --
pub const PROP_RENDERER_CREATE_NAME_STRING: &str = "SDL.renderer.create.name";
pub const PROP_RENDERER_CREATE_WINDOW_POINTER: &str = "SDL.renderer.create.window";
pub const PROP_RENDERER_CREATE_SURFACE_POINTER: &str = "SDL.renderer.create.surface";
pub const PROP_RENDERER_CREATE_OUTPUT_COLORSPACE_NUMBER: &str = "SDL.renderer.create.output_colorspace";
pub const PROP_RENDERER_CREATE_PRESENT_VSYNC_NUMBER: &str = "SDL.renderer.create.present_vsync";
pub const PROP_RENDERER_CREATE_GPU_DEVICE_POINTER: &str = "SDL.renderer.create.gpu.device";
pub const PROP_RENDERER_CREATE_GPU_SHADERS_SPIRV_BOOLEAN: &str = "SDL.renderer.create.gpu.shaders_spirv";
pub const PROP_RENDERER_CREATE_GPU_SHADERS_DXIL_BOOLEAN: &str = "SDL.renderer.create.gpu.shaders_dxil";
pub const PROP_RENDERER_CREATE_GPU_SHADERS_MSL_BOOLEAN: &str = "SDL.renderer.create.gpu.shaders_msl";
pub const PROP_RENDERER_CREATE_VULKAN_INSTANCE_POINTER: &str = "SDL.renderer.create.vulkan.instance";
pub const PROP_RENDERER_CREATE_VULKAN_PHYSICAL_DEVICE_POINTER: &str = "SDL.renderer.create.vulkan.physical_device";
pub const PROP_RENDERER_CREATE_VULKAN_DEVICE_POINTER: &str = "SDL.renderer.create.vulkan.device";
pub const PROP_RENDERER_CREATE_VULKAN_GRAPHICS_QUEUE_FAMILY_INDEX_NUMBER: &str = "SDL.renderer.create.vulkan.graphics_queue_family_index";
pub const PROP_RENDERER_CREATE_VULKAN_PRESENT_QUEUE_FAMILY_INDEX_NUMBER: &str = "SDL.renderer.create.vulkan.present_queue_family_index";
pub const PROP_RENDERER_CREATE_VULKAN_SURFACE_NUMBER: &str = "SDL.renderer.create.vulkan.surface";

// -- Renderer info --
pub const PROP_RENDERER_NAME_STRING: &str = "SDL.renderer.name";
pub const PROP_RENDERER_WINDOW_POINTER: &str = "SDL.renderer.window";
pub const PROP_RENDERER_SURFACE_POINTER: &str = "SDL.renderer.surface";
pub const PROP_RENDERER_GPU_DEVICE_POINTER: &str = "SDL.renderer.gpu.device";
pub const PROP_RENDERER_OUTPUT_COLORSPACE_NUMBER: &str = "SDL.renderer.output_colorspace";
pub const PROP_RENDERER_HDR_ENABLED_BOOLEAN: &str = "SDL.renderer.HDR_enabled";
pub const PROP_RENDERER_SDR_WHITE_POINT_FLOAT: &str = "SDL.renderer.SDR_white_point";
pub const PROP_RENDERER_HDR_HEADROOM_FLOAT: &str = "SDL.renderer.HDR_headroom";
pub const PROP_RENDERER_MAX_TEXTURE_SIZE_NUMBER: &str = "SDL.renderer.max_texture_size";
pub const PROP_RENDERER_TEXTURE_FORMATS_POINTER: &str = "SDL.renderer.texture_formats";
pub const PROP_RENDERER_TEXTURE_WRAPPING_BOOLEAN: &str = "SDL.renderer.texture_wrapping";
pub const PROP_RENDERER_VSYNC_NUMBER: &str = "SDL.renderer.vsync";
pub const PROP_RENDERER_VULKAN_INSTANCE_POINTER: &str = "SDL.renderer.vulkan.instance";
pub const PROP_RENDERER_VULKAN_PHYSICAL_DEVICE_POINTER: &str = "SDL.renderer.vulkan.physical_device";
pub const PROP_RENDERER_VULKAN_DEVICE_POINTER: &str = "SDL.renderer.vulkan.device";
pub const PROP_RENDERER_VULKAN_GRAPHICS_QUEUE_FAMILY_INDEX_NUMBER: &str = "SDL.renderer.vulkan.graphics_queue_family_index";
pub const PROP_RENDERER_VULKAN_PRESENT_QUEUE_FAMILY_INDEX_NUMBER: &str = "SDL.renderer.vulkan.present_queue_family_index";
pub const PROP_RENDERER_VULKAN_SURFACE_NUMBER: &str = "SDL.renderer.vulkan.surface";
pub const PROP_RENDERER_VULKAN_SWAPCHAIN_IMAGE_COUNT_NUMBER: &str = "SDL.renderer.vulkan.swapchain_image_count";
pub const PROP_RENDERER_D3D11_DEVICE_POINTER: &str = "SDL.renderer.d3d11.device";
pub const PROP_RENDERER_D3D11_SWAPCHAIN_POINTER: &str = "SDL.renderer.d3d11.swap_chain";
pub const PROP_RENDERER_D3D12_DEVICE_POINTER: &str = "SDL.renderer.d3d12.device";
pub const PROP_RENDERER_D3D12_COMMAND_QUEUE_POINTER: &str = "SDL.renderer.d3d12.command_queue";
pub const PROP_RENDERER_D3D12_SWAPCHAIN_POINTER: &str = "SDL.renderer.d3d12.swap_chain";
pub const PROP_RENDERER_D3D9_DEVICE_POINTER: &str = "SDL.renderer.d3d9.device";

// -- Surface --
pub const PROP_SURFACE_SDR_WHITE_POINT_FLOAT: &str = "SDL.surface.SDR_white_point";
pub const PROP_SURFACE_HDR_HEADROOM_FLOAT: &str = "SDL.surface.HDR_headroom";
pub const PROP_SURFACE_TONEMAP_OPERATOR_STRING: &str = "SDL.surface.tonemap";
pub const PROP_SURFACE_HOTSPOT_X_NUMBER: &str = "SDL.surface.hotspot.x";
pub const PROP_SURFACE_HOTSPOT_Y_NUMBER: &str = "SDL.surface.hotspot.y";
pub const PROP_SURFACE_ROTATION_FLOAT: &str = "SDL.surface.rotation";

// -- Thread creation --
pub const PROP_THREAD_CREATE_ENTRY_FUNCTION_POINTER: &str = "SDL.thread.create.entry_function";
pub const PROP_THREAD_CREATE_NAME_STRING: &str = "SDL.thread.create.name";
pub const PROP_THREAD_CREATE_USERDATA_POINTER: &str = "SDL.thread.create.userdata";
pub const PROP_THREAD_CREATE_STACKSIZE_NUMBER: &str = "SDL.thread.create.stacksize";

// -- Texture creation --
pub const PROP_TEXTURE_CREATE_WIDTH_NUMBER: &str = "SDL.texture.create.width";
pub const PROP_TEXTURE_CREATE_HEIGHT_NUMBER: &str = "SDL.texture.create.height";
pub const PROP_TEXTURE_CREATE_FORMAT_NUMBER: &str = "SDL.texture.create.format";
pub const PROP_TEXTURE_CREATE_ACCESS_NUMBER: &str = "SDL.texture.create.access";
pub const PROP_TEXTURE_CREATE_COLORSPACE_NUMBER: &str = "SDL.texture.create.colorspace";
pub const PROP_TEXTURE_CREATE_SDR_WHITE_POINT_FLOAT: &str = "SDL.texture.create.SDR_white_point";
pub const PROP_TEXTURE_CREATE_HDR_HEADROOM_FLOAT: &str = "SDL.texture.create.HDR_headroom";
pub const PROP_TEXTURE_CREATE_D3D11_TEXTURE_POINTER: &str = "SDL.texture.create.d3d11.texture";
pub const PROP_TEXTURE_CREATE_D3D11_TEXTURE_U_POINTER: &str = "SDL.texture.create.d3d11.texture_u";
pub const PROP_TEXTURE_CREATE_D3D11_TEXTURE_V_POINTER: &str = "SDL.texture.create.d3d11.texture_v";
pub const PROP_TEXTURE_CREATE_D3D12_TEXTURE_POINTER: &str = "SDL.texture.create.d3d12.texture";
pub const PROP_TEXTURE_CREATE_D3D12_TEXTURE_U_POINTER: &str = "SDL.texture.create.d3d12.texture_u";
pub const PROP_TEXTURE_CREATE_D3D12_TEXTURE_V_POINTER: &str = "SDL.texture.create.d3d12.texture_v";
pub const PROP_TEXTURE_CREATE_METAL_PIXELBUFFER_POINTER: &str = "SDL.texture.create.metal.pixelbuffer";
pub const PROP_TEXTURE_CREATE_OPENGL_TEXTURE_NUMBER: &str = "SDL.texture.create.opengl.texture";
pub const PROP_TEXTURE_CREATE_OPENGL_TEXTURE_U_NUMBER: &str = "SDL.texture.create.opengl.texture_u";
pub const PROP_TEXTURE_CREATE_OPENGL_TEXTURE_UV_NUMBER: &str = "SDL.texture.create.opengl.texture_uv";
pub const PROP_TEXTURE_CREATE_OPENGL_TEXTURE_V_NUMBER: &str = "SDL.texture.create.opengl.texture_v";
pub const PROP_TEXTURE_CREATE_OPENGLES2_TEXTURE_NUMBER: &str = "SDL.texture.create.opengles2.texture";
pub const PROP_TEXTURE_CREATE_OPENGLES2_TEXTURE_U_NUMBER: &str = "SDL.texture.create.opengles2.texture_u";
pub const PROP_TEXTURE_CREATE_OPENGLES2_TEXTURE_UV_NUMBER: &str = "SDL.texture.create.opengles2.texture_uv";
pub const PROP_TEXTURE_CREATE_OPENGLES2_TEXTURE_V_NUMBER: &str = "SDL.texture.create.opengles2.texture_v";
pub const PROP_TEXTURE_CREATE_VULKAN_TEXTURE_NUMBER: &str = "SDL.texture.create.vulkan.texture";
pub const PROP_TEXTURE_CREATE_VULKAN_LAYOUT_NUMBER: &str = "SDL.texture.create.vulkan.layout";
pub const PROP_TEXTURE_CREATE_GPU_TEXTURE_POINTER: &str = "SDL.texture.create.gpu.texture";
pub const PROP_TEXTURE_CREATE_GPU_TEXTURE_U_POINTER: &str = "SDL.texture.create.gpu.texture_u";
pub const PROP_TEXTURE_CREATE_GPU_TEXTURE_UV_POINTER: &str = "SDL.texture.create.gpu.texture_uv";
pub const PROP_TEXTURE_CREATE_GPU_TEXTURE_V_POINTER: &str = "SDL.texture.create.gpu.texture_v";
pub const PROP_TEXTURE_CREATE_PALETTE_POINTER: &str = "SDL.texture.create.palette";

// -- Texture info --
pub const PROP_TEXTURE_WIDTH_NUMBER: &str = "SDL.texture.width";
pub const PROP_TEXTURE_HEIGHT_NUMBER: &str = "SDL.texture.height";
pub const PROP_TEXTURE_FORMAT_NUMBER: &str = "SDL.texture.format";
pub const PROP_TEXTURE_ACCESS_NUMBER: &str = "SDL.texture.access";
pub const PROP_TEXTURE_COLORSPACE_NUMBER: &str = "SDL.texture.colorspace";
pub const PROP_TEXTURE_SDR_WHITE_POINT_FLOAT: &str = "SDL.texture.SDR_white_point";
pub const PROP_TEXTURE_HDR_HEADROOM_FLOAT: &str = "SDL.texture.HDR_headroom";
pub const PROP_TEXTURE_D3D11_TEXTURE_POINTER: &str = "SDL.texture.d3d11.texture";
pub const PROP_TEXTURE_D3D11_TEXTURE_U_POINTER: &str = "SDL.texture.d3d11.texture_u";
pub const PROP_TEXTURE_D3D11_TEXTURE_V_POINTER: &str = "SDL.texture.d3d11.texture_v";
pub const PROP_TEXTURE_D3D12_TEXTURE_POINTER: &str = "SDL.texture.d3d12.texture";
pub const PROP_TEXTURE_D3D12_TEXTURE_U_POINTER: &str = "SDL.texture.d3d12.texture_u";
pub const PROP_TEXTURE_D3D12_TEXTURE_V_POINTER: &str = "SDL.texture.d3d12.texture_v";
pub const PROP_TEXTURE_OPENGL_TEXTURE_NUMBER: &str = "SDL.texture.opengl.texture";
pub const PROP_TEXTURE_OPENGL_TEXTURE_U_NUMBER: &str = "SDL.texture.opengl.texture_u";
pub const PROP_TEXTURE_OPENGL_TEXTURE_UV_NUMBER: &str = "SDL.texture.opengl.texture_uv";
pub const PROP_TEXTURE_OPENGL_TEXTURE_V_NUMBER: &str = "SDL.texture.opengl.texture_v";
pub const PROP_TEXTURE_OPENGL_TEXTURE_TARGET_NUMBER: &str = "SDL.texture.opengl.target";
pub const PROP_TEXTURE_OPENGL_TEX_W_FLOAT: &str = "SDL.texture.opengl.tex_w";
pub const PROP_TEXTURE_OPENGL_TEX_H_FLOAT: &str = "SDL.texture.opengl.tex_h";
pub const PROP_TEXTURE_OPENGLES2_TEXTURE_NUMBER: &str = "SDL.texture.opengles2.texture";
pub const PROP_TEXTURE_OPENGLES2_TEXTURE_U_NUMBER: &str = "SDL.texture.opengles2.texture_u";
pub const PROP_TEXTURE_OPENGLES2_TEXTURE_UV_NUMBER: &str = "SDL.texture.opengles2.texture_uv";
pub const PROP_TEXTURE_OPENGLES2_TEXTURE_V_NUMBER: &str = "SDL.texture.opengles2.texture_v";
pub const PROP_TEXTURE_OPENGLES2_TEXTURE_TARGET_NUMBER: &str = "SDL.texture.opengles2.target";
pub const PROP_TEXTURE_VULKAN_TEXTURE_NUMBER: &str = "SDL.texture.vulkan.texture";
pub const PROP_TEXTURE_GPU_TEXTURE_POINTER: &str = "SDL.texture.gpu.texture";
pub const PROP_TEXTURE_GPU_TEXTURE_U_POINTER: &str = "SDL.texture.gpu.texture_u";
pub const PROP_TEXTURE_GPU_TEXTURE_UV_POINTER: &str = "SDL.texture.gpu.texture_uv";
pub const PROP_TEXTURE_GPU_TEXTURE_V_POINTER: &str = "SDL.texture.gpu.texture_v";

// -- Video global --
pub const PROP_GLOBAL_VIDEO_WAYLAND_WL_DISPLAY_POINTER: &str = "SDL.video.wayland.wl_display";

// -- Window creation --
pub const PROP_WINDOW_CREATE_ALWAYS_ON_TOP_BOOLEAN: &str = "SDL.window.create.always_on_top";
pub const PROP_WINDOW_CREATE_BORDERLESS_BOOLEAN: &str = "SDL.window.create.borderless";
pub const PROP_WINDOW_CREATE_COCOA_VIEW_POINTER: &str = "SDL.window.create.cocoa.view";
pub const PROP_WINDOW_CREATE_COCOA_WINDOW_POINTER: &str = "SDL.window.create.cocoa.window";
pub const PROP_WINDOW_CREATE_CONSTRAIN_POPUP_BOOLEAN: &str = "SDL.window.create.constrain_popup";
pub const PROP_WINDOW_CREATE_EMSCRIPTEN_CANVAS_ID_STRING: &str = "SDL.window.create.emscripten.canvas_id";
pub const PROP_WINDOW_CREATE_EMSCRIPTEN_KEYBOARD_ELEMENT_STRING: &str = "SDL.window.create.emscripten.keyboard_element";
pub const PROP_WINDOW_CREATE_EXTERNAL_GRAPHICS_CONTEXT_BOOLEAN: &str = "SDL.window.create.external_graphics_context";
pub const PROP_WINDOW_CREATE_FLAGS_NUMBER: &str = "SDL.window.create.flags";
pub const PROP_WINDOW_CREATE_FOCUSABLE_BOOLEAN: &str = "SDL.window.create.focusable";
pub const PROP_WINDOW_CREATE_FULLSCREEN_BOOLEAN: &str = "SDL.window.create.fullscreen";
pub const PROP_WINDOW_CREATE_HEIGHT_NUMBER: &str = "SDL.window.create.height";
pub const PROP_WINDOW_CREATE_HIDDEN_BOOLEAN: &str = "SDL.window.create.hidden";
pub const PROP_WINDOW_CREATE_HIGH_PIXEL_DENSITY_BOOLEAN: &str = "SDL.window.create.high_pixel_density";
pub const PROP_WINDOW_CREATE_MAXIMIZED_BOOLEAN: &str = "SDL.window.create.maximized";
pub const PROP_WINDOW_CREATE_MENU_BOOLEAN: &str = "SDL.window.create.menu";
pub const PROP_WINDOW_CREATE_METAL_BOOLEAN: &str = "SDL.window.create.metal";
pub const PROP_WINDOW_CREATE_MINIMIZED_BOOLEAN: &str = "SDL.window.create.minimized";
pub const PROP_WINDOW_CREATE_MODAL_BOOLEAN: &str = "SDL.window.create.modal";
pub const PROP_WINDOW_CREATE_MOUSE_GRABBED_BOOLEAN: &str = "SDL.window.create.mouse_grabbed";
pub const PROP_WINDOW_CREATE_OPENGL_BOOLEAN: &str = "SDL.window.create.opengl";
pub const PROP_WINDOW_CREATE_PARENT_POINTER: &str = "SDL.window.create.parent";
pub const PROP_WINDOW_CREATE_RESIZABLE_BOOLEAN: &str = "SDL.window.create.resizable";
pub const PROP_WINDOW_CREATE_TITLE_STRING: &str = "SDL.window.create.title";
pub const PROP_WINDOW_CREATE_TOOLTIP_BOOLEAN: &str = "SDL.window.create.tooltip";
pub const PROP_WINDOW_CREATE_TRANSPARENT_BOOLEAN: &str = "SDL.window.create.transparent";
pub const PROP_WINDOW_CREATE_UTILITY_BOOLEAN: &str = "SDL.window.create.utility";
pub const PROP_WINDOW_CREATE_VULKAN_BOOLEAN: &str = "SDL.window.create.vulkan";
pub const PROP_WINDOW_CREATE_WAYLAND_CREATE_EGL_WINDOW_BOOLEAN: &str = "SDL.window.create.wayland.create_egl_window";
pub const PROP_WINDOW_CREATE_WAYLAND_SURFACE_ROLE_CUSTOM_BOOLEAN: &str = "SDL.window.create.wayland.surface_role_custom";
pub const PROP_WINDOW_CREATE_WAYLAND_WL_SURFACE_POINTER: &str = "SDL.window.create.wayland.wl_surface";
pub const PROP_WINDOW_CREATE_WIDTH_NUMBER: &str = "SDL.window.create.width";
pub const PROP_WINDOW_CREATE_WIN32_HWND_POINTER: &str = "SDL.window.create.win32.hwnd";
pub const PROP_WINDOW_CREATE_WIN32_PIXEL_FORMAT_HWND_POINTER: &str = "SDL.window.create.win32.pixel_format_hwnd";
pub const PROP_WINDOW_CREATE_WINDOWSCENE_POINTER: &str = "SDL.window.create.uikit.windowscene";
pub const PROP_WINDOW_CREATE_X11_WINDOW_NUMBER: &str = "SDL.window.create.x11.window";
pub const PROP_WINDOW_CREATE_X_NUMBER: &str = "SDL.window.create.x";
pub const PROP_WINDOW_CREATE_Y_NUMBER: &str = "SDL.window.create.y";

// -- Window info --
pub const PROP_WINDOW_HDR_ENABLED_BOOLEAN: &str = "SDL.window.HDR_enabled";
pub const PROP_WINDOW_HDR_HEADROOM_FLOAT: &str = "SDL.window.HDR_headroom";
pub const PROP_WINDOW_SDR_WHITE_LEVEL_FLOAT: &str = "SDL.window.SDR_white_level";
pub const PROP_WINDOW_SHAPE_POINTER: &str = "SDL.window.shape";
pub const PROP_WINDOW_EMSCRIPTEN_CANVAS_ID_STRING: &str = "SDL.window.emscripten.canvas_id";
pub const PROP_WINDOW_EMSCRIPTEN_KEYBOARD_ELEMENT_STRING: &str = "SDL.window.emscripten.keyboard_element";

// -- Window platform pointers --
pub const PROP_WINDOW_ANDROID_SURFACE_POINTER: &str = "SDL.window.android.surface";
pub const PROP_WINDOW_ANDROID_WINDOW_POINTER: &str = "SDL.window.android.window";
pub const PROP_WINDOW_COCOA_WINDOW_POINTER: &str = "SDL.window.cocoa.window";
pub const PROP_WINDOW_COCOA_METAL_VIEW_TAG_NUMBER: &str = "SDL.window.cocoa.metal_view_tag";
pub const PROP_WINDOW_KMSDRM_DEVICE_INDEX_NUMBER: &str = "SDL.window.kmsdrm.dev_index";
pub const PROP_WINDOW_KMSDRM_DRM_FD_NUMBER: &str = "SDL.window.kmsdrm.drm_fd";
pub const PROP_WINDOW_KMSDRM_GBM_DEVICE_POINTER: &str = "SDL.window.kmsdrm.gbm_dev";
pub const PROP_WINDOW_OPENVR_OVERLAY_ID_NUMBER: &str = "SDL.window.openvr.overlay_id";
pub const PROP_WINDOW_UIKIT_WINDOW_POINTER: &str = "SDL.window.uikit.window";
pub const PROP_WINDOW_UIKIT_METAL_VIEW_TAG_NUMBER: &str = "SDL.window.uikit.metal_view_tag";
pub const PROP_WINDOW_UIKIT_OPENGL_FRAMEBUFFER_NUMBER: &str = "SDL.window.uikit.opengl.framebuffer";
pub const PROP_WINDOW_UIKIT_OPENGL_RENDERBUFFER_NUMBER: &str = "SDL.window.uikit.opengl.renderbuffer";
pub const PROP_WINDOW_UIKIT_OPENGL_RESOLVE_FRAMEBUFFER_NUMBER: &str = "SDL.window.uikit.opengl.resolve_framebuffer";
pub const PROP_WINDOW_VIVANTE_DISPLAY_POINTER: &str = "SDL.window.vivante.display";
pub const PROP_WINDOW_VIVANTE_SURFACE_POINTER: &str = "SDL.window.vivante.surface";
pub const PROP_WINDOW_VIVANTE_WINDOW_POINTER: &str = "SDL.window.vivante.window";
pub const PROP_WINDOW_WAYLAND_DISPLAY_POINTER: &str = "SDL.window.wayland.display";
pub const PROP_WINDOW_WAYLAND_SURFACE_POINTER: &str = "SDL.window.wayland.surface";
pub const PROP_WINDOW_WAYLAND_EGL_WINDOW_POINTER: &str = "SDL.window.wayland.egl_window";
pub const PROP_WINDOW_WAYLAND_VIEWPORT_POINTER: &str = "SDL.window.wayland.viewport";
pub const PROP_WINDOW_WAYLAND_XDG_POPUP_POINTER: &str = "SDL.window.wayland.xdg_popup";
pub const PROP_WINDOW_WAYLAND_XDG_POSITIONER_POINTER: &str = "SDL.window.wayland.xdg_positioner";
pub const PROP_WINDOW_WAYLAND_XDG_SURFACE_POINTER: &str = "SDL.window.wayland.xdg_surface";
pub const PROP_WINDOW_WAYLAND_XDG_TOPLEVEL_POINTER: &str = "SDL.window.wayland.xdg_toplevel";
pub const PROP_WINDOW_WAYLAND_XDG_TOPLEVEL_EXPORT_HANDLE_STRING: &str = "SDL.window.wayland.xdg_toplevel_export_handle";
pub const PROP_WINDOW_WIN32_HWND_POINTER: &str = "SDL.window.win32.hwnd";
pub const PROP_WINDOW_WIN32_HDC_POINTER: &str = "SDL.window.win32.hdc";
pub const PROP_WINDOW_WIN32_INSTANCE_POINTER: &str = "SDL.window.win32.instance";
pub const PROP_WINDOW_X11_DISPLAY_POINTER: &str = "SDL.window.x11.display";
pub const PROP_WINDOW_X11_SCREEN_NUMBER: &str = "SDL.window.x11.screen";
pub const PROP_WINDOW_X11_WINDOW_NUMBER: &str = "SDL.window.x11.window";
