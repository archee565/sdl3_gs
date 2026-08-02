use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU32, Ordering};

use sdl3_sys as sys;
use sys::*;

pub use gpu::SDL_GPUTextureCreateInfo;
pub use gpu::SDL_GPURasterizerState;
pub use gpu::SDL_GPUMultisampleState;
pub use gpu::SDL_GPUShaderFormat;
pub use gpu::SDL_GPUDepthStencilState;
pub use gpu::SDL_GPUShaderStage;
pub use gpu::SDL_GPULoadOp;
pub use gpu::SDL_GPUStoreOp;
pub use gpu::SDL_GPUPrimitiveType;
pub use gpu::SDL_GPUFillMode;
pub use gpu::SDL_GPUCullMode;
pub use gpu::SDL_GPUFrontFace;
pub use gpu::SDL_GPUSampleCount;
pub use gpu::SDL_GPUCompareOp;
pub use gpu::SDL_GPUStencilOp;
pub use gpu::SDL_GPUStencilOpState;
pub use gpu::SDL_GPUBlendFactor;
pub use gpu::SDL_GPUBlendOp;
pub use gpu::SDL_GPUColorComponentFlags;
pub use gpu::SDL_GPUVertexElementFormat;
pub use gpu::SDL_GPUVertexInputRate;
pub use gpu::SDL_GPUVertexAttribute;
pub use gpu::SDL_GPUVertexBufferDescription;
pub use gpu::SDL_GPUColorTargetBlendState;
pub use gpu::SDL_GPUColorTargetDescription;
pub use gpu::SDL_GPUTextureFormat;
pub use gpu::SDL_GPUBufferUsageFlags;
pub use gpu::SDL_GPUIndexElementSize;
pub use gpu::SDL_GPUTransferBufferUsage;
pub use gpu::SDL_GPUFilter;
pub use gpu::SDL_GPUSamplerAddressMode;
pub use gpu::SDL_GPUSamplerMipmapMode;
pub use gpu::SDL_GPUSamplerCreateInfo;
pub use gpu::SDL_GPUTextureUsageFlags;
pub use gpu::SDL_GPUTextureType;
pub use sys::pixels::SDL_FColor;
pub use sys::rect::SDL_Rect;
pub use sys::surface::SDL_FlipMode;
pub use gpu::SDL_GPUViewport;
pub use gpu::SDL_GPUPresentMode;
pub use gpu::SDL_GPUSwapchainComposition;


fn sdl_err() -> String {
    crate::sdl_get_error()
}

fn sdl_fail(context: &str) -> String {
    format!("{}: {}", context, sdl_err())
}

fn validate_sample_count(sample_count: gpu::SDL_GPUSampleCount) -> Result<(), String> {
    match sample_count {
        gpu::SDL_GPUSampleCount::_1
        | gpu::SDL_GPUSampleCount::_2
        | gpu::SDL_GPUSampleCount::_4
        | gpu::SDL_GPUSampleCount::_8 => Ok(()),
        _ => Err("invalid sample count: must be 1, 2, 4, or 8".into()),
    }
}

pub struct ColorTargetInfo {
    /// The texture that will be used as a color target by a render pass.
    pub texture: Texture,
    /// The mip level to use as a color target.
    pub mip_level: u32,
    /// The layer index or depth plane to use as a color target.
    pub layer_or_depth_plane: u32,
    /// The color to clear the color target to at the start of the render pass.
    pub clear_color: SDL_FColor,
    /// What is done with the contents of the color target at the beginning of the render pass.
    pub load_op: SDL_GPULoadOp,
    /// What is done with the results of the render pass.
    pub store_op: SDL_GPUStoreOp,
    /// The texture that will receive the results of a multisample resolve operation.
    pub resolve_texture: Option<Texture>,
    /// The mip level of the resolve texture to use for the resolve operation.
    pub resolve_mip_level: u32,
    /// The layer index of the resolve texture to use for the resolve operation.
    pub resolve_layer: u32,
    /// true cycles the texture if the texture is bound and load_op is not LOAD.
    pub cycle: bool,
    /// true cycles the resolve texture if the resolve texture is bound.
    pub cycle_resolve_texture: bool,
}

impl ColorTargetInfo {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            mip_level: 0,
            layer_or_depth_plane: 0,
            clear_color: SDL_FColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            load_op: SDL_GPULoadOp::default(),
            store_op: SDL_GPUStoreOp::default(),
            resolve_texture: None,
            resolve_mip_level: 0,
            resolve_layer: 0,
            cycle: false,
            cycle_resolve_texture: false,
        }
    }

    #[allow(deprecated)]
    pub(crate) fn to_raw(&self, _device: &Device) -> gpu::SDL_GPUColorTargetInfo {
        assert!(
            self.texture.inner.kind.get() != TextureKind::None,
            "cannot use a consumed swapchain texture as a color target"
        );
        if let Some(ref rt) = self.resolve_texture {
            assert!(
                rt.inner.kind.get() != TextureKind::None,
                "cannot use a consumed swapchain texture as a resolve target"
            );
        }
        gpu::SDL_GPUColorTargetInfo {
            texture: self.texture.raw(),
            mip_level: self.mip_level,
            layer_or_depth_plane: self.layer_or_depth_plane,
            clear_color: self.clear_color,
            load_op: self.load_op,
            store_op: self.store_op,
            resolve_texture: self.resolve_texture
                .as_ref()
                .map(|t| t.raw())
                .unwrap_or(std::ptr::null_mut()),
            resolve_mip_level: self.resolve_mip_level,
            resolve_layer: self.resolve_layer,
            cycle: self.cycle,
            cycle_resolve_texture: self.cycle_resolve_texture,
            padding1: 0,
            padding2: 0,
        }
    }
}

pub struct DepthStencilTargetInfo {
    /// The texture that will be used as the depth stencil target by the render pass.
    pub texture: Texture,
    /// The value to clear the depth component to at the beginning of the render pass.
    pub clear_depth: f32,
    /// What is done with the depth contents at the beginning of the render pass.
    pub load_op: SDL_GPULoadOp,
    /// What is done with the depth results of the render pass.
    pub store_op: SDL_GPUStoreOp,
    /// What is done with the stencil contents at the beginning of the render pass.
    pub stencil_load_op: SDL_GPULoadOp,
    /// What is done with the stencil results of the render pass.
    pub stencil_store_op: SDL_GPUStoreOp,
    /// true cycles the texture if the texture is bound and any load ops are not LOAD.
    pub cycle: bool,
    /// The value to clear the stencil component to at the beginning of the render pass.
    pub clear_stencil: u8,
    /// The mip level to use as the depth stencil target.
    pub mip_level: u8,
    /// The layer index to use as the depth stencil target.
    pub layer: u8,
}

impl DepthStencilTargetInfo {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            clear_depth: 1.0,
            load_op: SDL_GPULoadOp::default(),
            store_op: SDL_GPUStoreOp::default(),
            stencil_load_op: SDL_GPULoadOp::default(),
            stencil_store_op: SDL_GPUStoreOp::default(),
            cycle: false,
            clear_stencil: 0,
            mip_level: 0,
            layer: 0,
        }
    }

    pub(crate) fn to_raw(&self, _device: &Device) -> gpu::SDL_GPUDepthStencilTargetInfo {
        assert!(
            self.texture.inner.kind.get() != TextureKind::None,
            "cannot use a consumed swapchain texture as a depth-stencil target"
        );
        gpu::SDL_GPUDepthStencilTargetInfo {
            texture: self.texture.raw(),
            clear_depth: self.clear_depth,
            load_op: self.load_op,
            store_op: self.store_op,
            stencil_load_op: self.stencil_load_op,
            stencil_store_op: self.stencil_store_op,
            cycle: self.cycle,
            clear_stencil: self.clear_stencil,
            mip_level: self.mip_level,
            layer: self.layer,
        }
    }
}

/// A region of a texture, using a safe `Texture` handle instead of a raw pointer.
pub struct TextureRegion {
    pub texture: Texture,
    pub mip_level: u32,
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
    pub h: u32,
    pub d: u32,
}

impl TextureRegion {
    pub fn full(texture: Texture) -> Self {
        let (w, h) = texture.res();
        Self {
            texture,
            mip_level: 0,
            layer: 0,
            x: 0,
            y: 0,
            z: 0,
            w,
            h,
            d: 1,
        }
    }

    pub(crate) fn to_raw(&self) -> gpu::SDL_GPUTextureRegion {
        gpu::SDL_GPUTextureRegion {
            texture: self.texture.raw(),
            mip_level: self.mip_level,
            layer: self.layer,
            x: self.x,
            y: self.y,
            z: self.z,
            w: self.w,
            h: self.h,
            d: self.d,
        }
    }
}

/// A region of a texture used in a blit operation.
pub struct BlitRegion {
    /// The texture.
    pub texture: Texture,
    /// The mip level index of the region.
    pub mip_level: u32,
    /// The layer index or depth plane of the region.
    pub layer_or_depth_plane: u32,
    /// The left offset of the region.
    pub x: u32,
    /// The top offset of the region.
    pub y: u32,
    /// The width of the region.
    pub w: u32,
    /// The height of the region.
    pub h: u32,
}

impl BlitRegion {
    /// Create a blit region covering the full texture.
    pub fn full(texture: Texture, w: u32, h: u32) -> Self {
        Self {
            texture,
            mip_level: 0,
            layer_or_depth_plane: 0,
            x: 0,
            y: 0,
            w,
            h,
        }
    }

    pub(crate) fn to_raw(&self, _device: &Device) -> gpu::SDL_GPUBlitRegion {
        gpu::SDL_GPUBlitRegion {
            texture: self.texture.raw(),
            mip_level: self.mip_level,
            layer_or_depth_plane: self.layer_or_depth_plane,
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

/// Parameters for a blit (texture copy with optional scaling/filtering).
pub struct BlitInfo {
    /// The source region for the blit.
    pub source: BlitRegion,
    /// The destination region for the blit.
    pub destination: BlitRegion,
    /// What is done with the contents of the destination before the blit.
    pub load_op: SDL_GPULoadOp,
    /// The color to clear the destination region to before the blit. Ignored if load_op is not CLEAR.
    pub clear_color: SDL_FColor,
    /// The flip mode for the source region.
    pub flip_mode: SDL_FlipMode,
    /// The filter mode used when blitting.
    pub filter: SDL_GPUFilter,
    /// true cycles the destination texture if it is already bound.
    pub cycle: bool,
}

impl BlitInfo {
    /// Create a BlitInfo with sensible defaults (DONT_CARE load, no flip, nearest filter).
    pub fn new(source: BlitRegion, destination: BlitRegion) -> Self {
        Self {
            source,
            destination,
            load_op: SDL_GPULoadOp::DONT_CARE,
            clear_color: SDL_FColor { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            flip_mode: SDL_FlipMode::NONE,
            filter: SDL_GPUFilter::NEAREST,
            cycle: false,
        }
    }

    #[allow(deprecated)]
    pub(crate) fn to_raw(&self, device: &Device) -> gpu::SDL_GPUBlitInfo {
        gpu::SDL_GPUBlitInfo {
            source: self.source.to_raw(device),
            destination: self.destination.to_raw(device),
            load_op: self.load_op,
            clear_color: self.clear_color,
            flip_mode: self.flip_mode,
            filter: self.filter,
            cycle: self.cycle,
            padding1: 0,
            padding2: 0,
            padding3: 0,
        }
    }
}

pub struct ShaderCreateInfo<'a> {
    /// The shader bytecode.
    pub code: &'a [u8],
    /// The entry point function name.
    pub entrypoint: &'a str,
    /// The format of the shader code.
    pub format: SDL_GPUShaderFormat,
    /// The stage the shader program corresponds to.
    pub stage: SDL_GPUShaderStage,
    /// The number of samplers defined in the shader.
    pub num_samplers: u32,
    /// The number of storage textures defined in the shader.
    pub num_storage_textures: u32,
    /// The number of storage buffers defined in the shader.
    pub num_storage_buffers: u32,
    /// The number of uniform buffers defined in the shader.
    pub num_uniform_buffers: u32,
}

struct DeviceInner {
    raw: *mut gpu::SDL_GPUDevice,
    upload_transfer_buffer: RefCell<Option<GPUTransferBuffer>>,
    cmd_buf_count: AtomicU32,
    pending_transfer_buffers: RefCell<Vec<GPUTransferBuffer>>,
    window: Option<Rc<crate::window::Window>>,
}

pub struct Device
{
    inner: Rc<DeviceInner>,
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl Default for Device {
    fn default() -> Self {
        Device {
            inner: Rc::new(DeviceInner {
                raw: std::ptr::null_mut(),
                upload_transfer_buffer: RefCell::new(None),
                cmd_buf_count: AtomicU32::new(0),
                pending_transfer_buffers: RefCell::new(Vec::new()),
                window: None,
            }),
        }
    }
}

impl Device {

    pub fn get_swapchain_texture_format(&self) -> SDL_GPUTextureFormat {
        let window = self.inner.window.as_ref().expect("Device has no window");
        unsafe { gpu::SDL_GetGPUSwapchainTextureFormat(        self.raw(), window.raw()) }
    }

    pub fn get_shader_formats(&self) -> SDL_GPUShaderFormat {
        unsafe { gpu::SDL_GetGPUShaderFormats(        self.raw()) }
    }

    /// Create a GPU transfer buffer for upload/download staging.
    pub fn create_transfer_buffer(
        &self,
        usage: gpu::SDL_GPUTransferBufferUsage,
        size: u32,
    ) -> Result<GPUTransferBuffer, String> {
        let tb_info = gpu::SDL_GPUTransferBufferCreateInfo {
            usage,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUTransferBuffer(self.raw(), &tb_info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUTransferBuffer"));
            }
            Ok(GPUTransferBuffer {
                inner: Rc::new(TransferBufferData {
                    raw, size, device: Rc::downgrade(&self.inner),
                }),
            })
        }
    }

    pub fn get_driver_name(&self) -> String
    {
        unsafe
        {
            std::ffi::CStr::from_ptr(sys::gpu::SDL_GetGPUDeviceDriver(        self.raw())).to_string_lossy().to_string()
        }
    }

    /// Get the device properties for this device
    pub fn get_device_properties(&self) -> crate::properties::Properties {
        unsafe
        {
            crate::properties::Properties::from_raw(sys::gpu::SDL_GetGPUDeviceProperties(        self.raw()))
        }
    }


    pub fn acquire_command_buffer(&self) -> Result<CommandBuffer, String> {
        unsafe {
            let raw = gpu::SDL_AcquireGPUCommandBuffer(        self.raw());
            if raw.is_null() {
                return Err(sdl_fail("SDL_AcquireGPUCommandBuffer"));
            }
            self.inner().cmd_buf_count.fetch_add(1, Ordering::Relaxed);
            Ok(CommandBuffer { inner: raw, device: self.clone(), submitted: false, pass_active: Cell::new(false), swapchain_texture: RefCell::new(None) })
        }
    }

    /// Called when a command buffer is submitted or cancelled.
    /// When no command buffers remain in flight, releases all deferred transfer buffers.
    fn on_command_buffer_done(&self) {
        let di = self.inner();
        let prev = di.cmd_buf_count.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(prev > 0, "command buffer count underflow");
        if prev == 1 {
            di.pending_transfer_buffers.borrow_mut().clear();
        }
    }

    pub fn wait_for_swapchain(&self) -> Result<(), String> {
        let window = self.inner.window.as_ref()
            .ok_or_else(|| String::from("Device has no window"))?;
        unsafe {
            if !gpu::SDL_WaitForGPUSwapchain(        self.raw(), window.raw()) {
                return Err(sdl_fail("SDL_WaitForGPUSwapchain"));
            }
        }
        Ok(())
    }

    /// Wait for a single fence.
    fn wait_for_fence(&self, fence: &Fence) -> Result<(), String> {
        if !fence.is_valid() { return Err( String::from("Invalid fence"));  }
        unsafe {
            if !gpu::SDL_WaitForGPUFences(        self.raw(), true, &fence.inner.raw, 1) {
                return Err(sdl_fail("SDL_WaitForGPUFences"));
            }
        }
        Ok(())
    }

    /// Query a fence (non-blocking).
    pub fn query_fence(&self, fence: &Fence) -> bool {
        assert!(fence.is_valid());
        unsafe {
            gpu::SDL_QueryGPUFence(        self.raw(), fence.inner.raw)
        }
    }

    /// Wait for a fence and then release it.
    pub fn wait_for_fence_then_release(&self, fence: Fence) -> Result<(), String> {
        self.wait_for_fence(&fence)?;
        drop(fence);
        Ok(())
    }

    /// Wait for multiple fences. If `wait_all` is true, waits for all fences;
    /// otherwise waits for any one fence.
    pub fn wait_for_fences(&self, fences: &[Fence], wait_all: bool) -> Result<(), String> {
        if fences.is_empty() {
            return Ok(());
        }
        let mut ptrs: Vec<*mut gpu::SDL_GPUFence> = Vec::with_capacity(fences.len());
        for f in fences {
            ptrs.push(f.inner.raw);
        }
        unsafe {
            if !gpu::SDL_WaitForGPUFences(        self.raw(), wait_all, ptrs.as_ptr(), ptrs.len() as u32) {
                return Err(sdl_fail("SDL_WaitForGPUFences"));
            }
        }
        Ok(())
    }

    /// Block until all GPU work is complete. Equivalent to SDL_WaitForGPUIdle.
    /// Expensive — use only for debugging synchronization issues.
    pub fn wait_idle(&self) -> Result<(), String> {
        unsafe {
            if !gpu::SDL_WaitForGPUIdle(        self.raw()) {
                return Err(sdl_fail("SDL_WaitForGPUIdle"));
            }
        }
        Ok(())
    }

    fn inner(&self) -> &DeviceInner {
        &self.inner
    }

    pub(crate) fn raw(&self) -> *mut gpu::SDL_GPUDevice {
        self.inner.raw
    }

    pub fn get_window(&self) -> Option<&crate::window::Window>
    {
        self.inner.window.as_deref()
    }

    pub fn new(format : gpu::SDL_GPUShaderFormat, window : Option<crate::window::Window>, properties : Option<sys::properties::SDL_PropertiesID>) -> Result<Self,String>
    {
        unsafe {
            let sys_device = if let Some(props) = properties {
                if format & gpu::SDL_GPUShaderFormat::DXIL != gpu::SDL_GPUShaderFormat(0) {
                    sys::properties::SDL_SetBooleanProperty(props, gpu::SDL_PROP_GPU_DEVICE_CREATE_SHADERS_DXIL_BOOLEAN, true);
                }
                if format & gpu::SDL_GPUShaderFormat::SPIRV != gpu::SDL_GPUShaderFormat(0) {
                    sys::properties::SDL_SetBooleanProperty(props, gpu::SDL_PROP_GPU_DEVICE_CREATE_SHADERS_SPIRV_BOOLEAN, true);
                }
                if format & gpu::SDL_GPUShaderFormat::MSL != gpu::SDL_GPUShaderFormat(0) {
                    sys::properties::SDL_SetBooleanProperty(props, gpu::SDL_PROP_GPU_DEVICE_CREATE_SHADERS_MSL_BOOLEAN, true);
                }
                gpu::SDL_CreateGPUDeviceWithProperties(props)
            } else {
                gpu::SDL_CreateGPUDevice(
                    format,
                    true,
                    std::ptr::null(),
                )
            };

            if sys_device.is_null()
            {
                return Err(sdl_fail("SDL_CreateGPUDevice"));
            }

            if let Some(window) = &window
            {
                gpu::SDL_ClaimWindowForGPUDevice(sys_device,window.raw());
            }
            
            let inner = DeviceInner {
                raw: sys_device,
                window: window.map(Rc::new),
                upload_transfer_buffer: RefCell::new(None),
                cmd_buf_count: AtomicU32::new(0),
                pending_transfer_buffers: RefCell::new(Vec::new()),
            };
            let inner_rc = Rc::new(inner);
            Ok(Device { inner: inner_rc })
        }
        
    }

    /// Release the window from the GPU device. Must be called on Android
    /// when the app enters the background (`SDL_EVENT_DID_ENTER_BACKGROUND`).
    pub fn release_window(&self) {
        if let Some(window) = &self.inner.window {
            unsafe { gpu::SDL_ReleaseWindowFromGPUDevice(        self.raw(), window.raw()); }
        }
    }

    /// Claim the window for the GPU device. Must be called on Android
    /// when the app enters the foreground (`SDL_EVENT_WILL_ENTER_FOREGROUND`).
    pub fn claim_window(&self) {
        if let Some(window) = &self.inner.window {
            unsafe { gpu::SDL_ClaimWindowForGPUDevice(        self.raw(), window.raw()); }
        }
    }

    /// Set the swapchain composition and present mode.
    /// Returns false if no window is associated with the device.
    pub fn set_swapchain_parameters(
        &self,
        swapchain_composition: gpu::SDL_GPUSwapchainComposition,
        present_mode: gpu::SDL_GPUPresentMode,
    ) -> bool {
        if let Some(window) = &self.inner.window {
            unsafe {
                gpu::SDL_SetGPUSwapchainParameters(        self.raw(), window.raw(), swapchain_composition, present_mode)
            }
        } else {
            false
        }
    }

    /// Configure the maximum number of frames that can be pending on the GPU.
    ///
    /// Default is 2, valid range is 1–3. Lower values reduce latency at the
    /// expense of throughput; higher values increase throughput at the expense
    /// of latency.
    ///
    /// Returns true on success, false on error.
    pub fn set_allowed_frames_in_flight(&self, allowed_frames_in_flight: u32) -> bool {
        unsafe {
            gpu::SDL_SetGPUAllowedFramesInFlight(        self.raw(), allowed_frames_in_flight)
        }
    }

    pub fn create_texture(&self, info: &gpu::SDL_GPUTextureCreateInfo) -> Result<Texture, String> {
        validate_sample_count(info.sample_count)?;
        unsafe {
            let raw = gpu::SDL_CreateGPUTexture(        self.raw(), info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUTexture"));
            }
            Ok(Texture {
                inner: Rc::new(TextureData {
                    raw,
                    res: (info.width, info.height),
                    device: Rc::downgrade(&self.inner),
                    kind: Cell::new(TextureKind::Regular),
                }),
            })
        }
    }


    pub fn create_shader(&self, info: &ShaderCreateInfo) -> Result<Shader, String> {
        let entrypoint = std::ffi::CString::new(info.entrypoint)
            .map_err(|_| String::from("entrypoint contains interior nul byte"))?;
        let raw_info = gpu::SDL_GPUShaderCreateInfo {
            code_size: info.code.len(),
            code: info.code.as_ptr(),
            entrypoint: entrypoint.as_ptr(),
            format: info.format,
            stage: info.stage,
            num_samplers: info.num_samplers,
            num_storage_textures: info.num_storage_textures,
            num_storage_buffers: info.num_storage_buffers,
            num_uniform_buffers: info.num_uniform_buffers,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUShader(self.raw(), &raw_info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUShader"));
            }
            Ok(Shader {
                inner: Rc::new(ShaderData { raw, device: Rc::downgrade(&self.inner) }),
            })
        }
    }


    #[allow(deprecated)]
    pub fn create_graphics_pipeline(&self, info: &GraphicsPipelineCreateInfo<'_>) -> Result<GraphicsPipeline, String> {
        validate_sample_count(info.multisample_state.sample_count)?;
        let vertex_shader_raw = info.vertex_shader.raw();
        let fragment_shader_raw = info.fragment_shader.raw();
        let raw_info = gpu::SDL_GPUGraphicsPipelineCreateInfo {
            vertex_shader: vertex_shader_raw,
            fragment_shader: fragment_shader_raw,
            vertex_input_state: gpu::SDL_GPUVertexInputState {
                vertex_buffer_descriptions: if info.vertex_buffer_descriptions.is_empty() {
                    std::ptr::null()
                } else {
                    info.vertex_buffer_descriptions.as_ptr()
                },
                num_vertex_buffers: info.vertex_buffer_descriptions.len() as u32,
                vertex_attributes: if info.vertex_attributes.is_empty() {
                    std::ptr::null()
                } else {
                    info.vertex_attributes.as_ptr()
                },
                num_vertex_attributes: info.vertex_attributes.len() as u32,
            },
            primitive_type: info.primitive_type,
            rasterizer_state: info.rasterizer_state,
            multisample_state: info.multisample_state,
            depth_stencil_state: info.depth_stencil_state,
            target_info: gpu::SDL_GPUGraphicsPipelineTargetInfo {
                color_target_descriptions: if info.color_target_descriptions.is_empty() {
                    std::ptr::null()
                } else {
                    info.color_target_descriptions.as_ptr()
                },
                num_color_targets: info.color_target_descriptions.len() as u32,
                depth_stencil_format: info.depth_stencil_format,
                has_depth_stencil_target: info.has_depth_stencil_target,
                padding1: 0,
                padding2: 0,
                padding3: 0,
            },
            props: sys::properties::SDL_PropertiesID(0),
        };

        unsafe {
            let raw = gpu::SDL_CreateGPUGraphicsPipeline(        self.raw(), &raw_info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUGraphicsPipeline"));
            }
            Ok(GraphicsPipeline {
                inner: Rc::new(GraphicsPipelineData { raw, device: Rc::downgrade(&self.inner) }),
            })
        }
    }


    pub fn create_compute_pipeline(&self, info: &ComputePipelineCreateInfo) -> Result<ComputePipeline, String> {
        let entrypoint = std::ffi::CString::new(info.entrypoint)
            .map_err(|_| String::from("entrypoint contains interior nul byte"))?;
        let raw_info = gpu::SDL_GPUComputePipelineCreateInfo {
            code_size: info.code.len(),
            code: info.code.as_ptr(),
            entrypoint: entrypoint.as_ptr(),
            format: info.format,
            num_samplers: info.num_samplers,
            num_readonly_storage_textures: info.num_readonly_storage_textures,
            num_readonly_storage_buffers: info.num_readonly_storage_buffers,
            num_readwrite_storage_textures: info.num_readwrite_storage_textures,
            num_readwrite_storage_buffers: info.num_readwrite_storage_buffers,
            num_uniform_buffers: info.num_uniform_buffers,
            threadcount_x: info.threadcount_x,
            threadcount_y: info.threadcount_y,
            threadcount_z: info.threadcount_z,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUComputePipeline(        self.raw(), &raw_info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUComputePipeline"));
            }
            Ok(ComputePipeline {
                inner: Rc::new(ComputePipelineData { raw, device: Rc::downgrade(&self.inner) }),
            })
        }
    }

    pub fn create_buffer(&self, usage: SDL_GPUBufferUsageFlags, size: u32) -> Result<GPUBuffer, String> {
        let newbuf = self.create_buffer_sub(usage, size)?;
        let zerodata = vec![0u8; size as usize];
        newbuf.upload(None, 0, &zerodata)?;
        Ok(newbuf)
    }

    fn create_buffer_sub(&self, usage: SDL_GPUBufferUsageFlags, size: u32) -> Result<GPUBuffer, String> {
        let info = gpu::SDL_GPUBufferCreateInfo {
            usage,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUBuffer(self.raw(), &info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUBuffer"));
            }
            Ok(GPUBuffer {
                inner: Rc::new(GPUBufferData { raw: Cell::new(raw), size: Cell::new(size), usage, device: Rc::downgrade(&self.inner) }),
            })
        }
    }


    pub fn create_sampler(&self, info: &gpu::SDL_GPUSamplerCreateInfo) -> Result<Sampler, String> {
        unsafe {
            let raw = gpu::SDL_CreateGPUSampler(self.raw(), info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUSampler"));
            }
            Ok(Sampler {
                inner: Rc::new(SamplerData { raw, device: Rc::downgrade(&self.inner) }),
            })
        }
    }
}

impl DeviceInner {
    /// Ensure the internal upload transfer buffer is at least `size` bytes.
    fn ensure_upload_transfer_buffer(&self, size: u32, device_weak: &Weak<DeviceInner>) -> Result<GPUTransferBuffer, String> {
        if let Some(ref cached) = *self.upload_transfer_buffer.borrow() {
            if cached.size() >= size {
                return Ok(cached.clone());
            }
        }
        // Defer release of the old buffer until no command buffers are in flight.
        if let Some(old) = self.upload_transfer_buffer.borrow_mut().take() {
            self.pending_transfer_buffers.borrow_mut().push(old);
        }
        let tb_info = gpu::SDL_GPUTransferBufferCreateInfo {
            usage: gpu::SDL_GPUTransferBufferUsage::UPLOAD,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUTransferBuffer(self.raw, &tb_info);
            if raw.is_null() {
                *self.upload_transfer_buffer.borrow_mut() = None;
                return Err(sdl_fail("SDL_CreateGPUTransferBuffer"));
            }
            let tb = GPUTransferBuffer {
                inner: Rc::new(TransferBufferData {
                    raw, size, device: device_weak.clone(),
                }),
            };
            *self.upload_transfer_buffer.borrow_mut() = Some(tb.clone());
            Ok(tb)
        }
    }

    /// Stage upload data into the internal transfer buffer (map, copy, unmap).
    fn stage_upload(&self, data: &[u8], device_weak: &Weak<DeviceInner>) -> Result<GPUTransferBuffer, String> {
        let tb = self.ensure_upload_transfer_buffer(data.len() as u32, device_weak)?;
        tb.write(data)?;
        Ok(tb)
    }

    /// Run a closure on a copy pass. Uses the provided pass, or creates a
    /// temporary command buffer + copy pass, runs the closure, and submits.
    fn with_copy_pass(
        &self,
        copy_pass: Option<&CopyPass>,
        f: impl FnOnce(*mut gpu::SDL_GPUCopyPass),
    ) -> Result<(), String> {
        if let Some(pass) = copy_pass {
            f(pass.inner);
            return Ok(());
        }
        unsafe {
            let cmd = gpu::SDL_AcquireGPUCommandBuffer(self.raw);
            if cmd.is_null() {
                return Err(sdl_fail("SDL_AcquireGPUCommandBuffer"));
            }
            let tmp_pass = gpu::SDL_BeginGPUCopyPass(cmd);
            if tmp_pass.is_null() {
                gpu::SDL_CancelGPUCommandBuffer(cmd);
                return Err(sdl_fail("SDL_BeginGPUCopyPass"));
            }
            f(tmp_pass);
            gpu::SDL_EndGPUCopyPass(tmp_pass);
            if !gpu::SDL_SubmitGPUCommandBuffer(cmd) {
                return Err(sdl_fail("SDL_SubmitGPUCommandBuffer"));
            }
        }
        Ok(())
    }

    /// Create a download transfer buffer of the given size.
    fn create_download_transfer_buffer(&self, size: u32, device_weak: &Weak<DeviceInner>) -> Result<GPUTransferBuffer, String> {
        let tb_info = gpu::SDL_GPUTransferBufferCreateInfo {
            usage: gpu::SDL_GPUTransferBufferUsage::DOWNLOAD,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUTransferBuffer(self.raw, &tb_info);
            if raw.is_null() {
                return Err(sdl_fail("SDL_CreateGPUTransferBuffer (download)"));
            }
            Ok(GPUTransferBuffer {
                inner: Rc::new(TransferBufferData {
                    raw, size, device: device_weak.clone(),
                }),
            })
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureKind { Regular, Swapchain, None }

pub(crate) struct TextureData {
    pub(crate) raw: *mut gpu::SDL_GPUTexture,
    pub(crate) res: (u32, u32),
    device: Weak<DeviceInner>,
    pub(crate) kind: Cell<TextureKind>,
}

impl Drop for TextureData {
    fn drop(&mut self) {
        if self.kind.get() != TextureKind::Regular {
            return;
        }
        match self.device.upgrade() {
            Some(di) => unsafe {
                gpu::SDL_ReleaseGPUTexture(di.raw, self.raw);
            },
            None => {
                #[cfg(feature = "verbose")]
                ::log::warn!("Texture dropped after Device was destroyed (leaking SDL resource)");
            },
        }
    }
}

pub(crate) struct ShaderData {
    pub(crate) raw: *mut gpu::SDL_GPUShader,
    device: Weak<DeviceInner>,
}

impl Drop for ShaderData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUShader(di.raw, self.raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("Shader dropped after device was destroyed (leak)");
        }
    }
}

pub(crate) struct GraphicsPipelineData {
    pub(crate) raw: *mut gpu::SDL_GPUGraphicsPipeline,
    device: Weak<DeviceInner>,
}

impl Drop for GraphicsPipelineData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUGraphicsPipeline(di.raw, self.raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("GraphicsPipeline dropped after device was destroyed (leak)");
        }
    }
}

pub(crate) struct ComputePipelineData {
    pub(crate) raw: *mut gpu::SDL_GPUComputePipeline,
    device: Weak<DeviceInner>,
}

impl Drop for ComputePipelineData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUComputePipeline(di.raw, self.raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("ComputePipeline dropped after device was destroyed (leak)");
        }
    }
}

pub(crate) struct GPUBufferData {
    pub(crate) raw: Cell<*mut gpu::SDL_GPUBuffer>,
    pub(crate) size: Cell<u32>,
    pub(crate) usage: SDL_GPUBufferUsageFlags,
    device: Weak<DeviceInner>,
}

impl Drop for GPUBufferData {
    fn drop(&mut self) {
        let raw = self.raw.get();
        if raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUBuffer(di.raw, raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("GPUBuffer dropped after device was destroyed (leak)");
        }
    }
}

pub(crate) struct SamplerData {
    pub(crate) raw: *mut gpu::SDL_GPUSampler,
    device: Weak<DeviceInner>,
}

impl Drop for SamplerData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUSampler(di.raw, self.raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("Sampler dropped after device was destroyed (leak)");
        }
    }
}

pub(crate) struct FenceData {
    pub(crate) raw: *mut gpu::SDL_GPUFence,
    device: Weak<DeviceInner>,
}

impl Drop for FenceData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe {
                gpu::SDL_ReleaseGPUFence(di.raw, self.raw);
            }
        } else if cfg!(feature = "verbose") {
            ::log::warn!("Fence dropped after device was destroyed (leak)");
        }
    }
}

/// A GPU texture that is automatically released when dropped.
#[derive(Clone)]
pub struct Texture {
    pub(crate) inner: Rc<TextureData>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("raw", &(self.inner.raw as usize))
            .field("res", &self.inner.res)
            .finish()
    }
}

impl Texture {
    /// Upload data to the full texture. Uses the internal Weak ref for device access.
    pub fn upload(&self, data: &[u8]) -> Result<(), String> {
        let weak = self.inner.device.clone();
        let di = weak.upgrade().ok_or("Texture::upload: device dropped")?;
        let res = self.inner.res;
        let transfer = di.stage_upload(data, &weak)?;
        let src = gpu::SDL_GPUTextureTransferInfo {
            transfer_buffer: transfer.raw(),
            offset: 0,
            pixels_per_row: 0,
            rows_per_layer: 0,
        };
        let dst = gpu::SDL_GPUTextureRegion {
            texture: self.inner.raw,
            mip_level: 0,
            layer: 0,
            x: 0,
            y: 0,
            z: 0,
            w: res.0,
            h: res.1,
            d: 1,
        };
        di.with_copy_pass(None, |pass| unsafe {
            gpu::SDL_UploadToGPUTexture(pass, &src, &dst, true);
        })
    }

    /// Upload data to a sub-region of the texture.
    pub fn upload_region(
        &self,
        copy_pass: Option<&CopyPass>,
        region: &TextureRegion,
        data: &[u8],
    ) -> Result<(), String> {
        let weak = self.inner.device.clone();
        let di = weak.upgrade().ok_or("Texture::upload_region: device dropped")?;
        let transfer = di.stage_upload(data, &weak)?;
        let src = gpu::SDL_GPUTextureTransferInfo {
            transfer_buffer: transfer.raw(),
            offset: 0,
            pixels_per_row: 0,
            rows_per_layer: 0,
        };
        let dst = region.to_raw();
        di.with_copy_pass(copy_pass, |pass| unsafe {
            gpu::SDL_UploadToGPUTexture(pass, &src, &dst, true);
        })
    }
}

impl Default for Texture
{
    fn default() -> Self {
        Texture::none()
    }
}

impl Texture {
    pub fn none() -> Texture {
        Texture {
            inner: Rc::new(TextureData {
                raw: std::ptr::null_mut(), res: (0, 0), device: Weak::new(), kind: Cell::new(TextureKind::None),
            }),
        }
    }

    pub fn is_valid(&self) -> bool
    {
        self.inner.kind.get() != TextureKind::None
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUTexture {
        self.inner.raw
    }

    pub fn res(&self) -> (u32, u32) {
        self.inner.res
    }
}

thread_local! {
    static NONE_SHADER: Shader = Shader {
        inner: Rc::new(ShaderData { raw: std::ptr::null_mut(), device: Weak::new() }),
    };
}

/// A GPU shader that is automatically released when dropped.
#[derive(Clone)]
pub struct Shader {
    pub(crate) inner: Rc<ShaderData>,
}

impl std::fmt::Debug for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shader")
            .field("raw", &(self.inner.raw as usize))
            .finish()
    }
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.inner) == Rc::as_ptr(&other.inner)
    }
}

impl Eq for Shader {}

impl std::hash::Hash for Shader {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.inner).hash(state);
    }
}

impl Default for Shader {
    fn default() -> Self {
        Shader::none()
    }
}

impl Shader {
    pub fn none() -> Shader {
        NONE_SHADER.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUShader {
        self.inner.raw
    }
}

thread_local! {
    static NONE_GRAPHICS_PIPELINE: GraphicsPipeline = GraphicsPipeline {
        inner: Rc::new(GraphicsPipelineData { raw: std::ptr::null_mut(), device: Weak::new() }),
    };
}

/// Handle to a graphics pipeline that is automatically released when dropped.
#[derive(Clone)]
pub struct GraphicsPipeline {
    pub(crate) inner: Rc<GraphicsPipelineData>,
}

impl std::fmt::Debug for GraphicsPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphicsPipeline")
            .field("raw", &(self.inner.raw as usize))
            .finish()
    }
}

impl PartialEq for GraphicsPipeline {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.inner) == Rc::as_ptr(&other.inner)
    }
}

impl Eq for GraphicsPipeline {}

impl std::hash::Hash for GraphicsPipeline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.inner).hash(state);
    }
}

impl Default for GraphicsPipeline {
    fn default() -> Self {
        GraphicsPipeline::none()
    }
}

impl GraphicsPipeline {
    pub fn none() -> GraphicsPipeline {
        NONE_GRAPHICS_PIPELINE.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUGraphicsPipeline {
        self.inner.raw
    }
}

thread_local! {
    static NONE_COMPUTE_PIPELINE: ComputePipeline = ComputePipeline {
        inner: Rc::new(ComputePipelineData { raw: std::ptr::null_mut(), device: Weak::new() }),
    };
}

/// Handle to a compute pipeline that is automatically released when dropped.
#[derive(Clone)]
pub struct ComputePipeline {
    pub(crate) inner: Rc<ComputePipelineData>,
}

impl std::fmt::Debug for ComputePipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputePipeline")
            .field("raw", &(self.inner.raw as usize))
            .finish()
    }
}

impl PartialEq for ComputePipeline {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.inner) == Rc::as_ptr(&other.inner)
    }
}

impl Eq for ComputePipeline {}

impl std::hash::Hash for ComputePipeline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.inner).hash(state);
    }
}

impl Default for ComputePipeline {
    fn default() -> Self {
        ComputePipeline::none()
    }
}

impl ComputePipeline {
    pub fn none() -> ComputePipeline {
        NONE_COMPUTE_PIPELINE.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUComputePipeline {
        self.inner.raw
    }
}



thread_local! {
    static NONE_GPUBUFFER: GPUBuffer = GPUBuffer {
        inner: Rc::new(GPUBufferData { raw: Cell::new(std::ptr::null_mut()), size: Cell::new(0), usage: SDL_GPUBufferUsageFlags(0), device: Weak::new() }),
    };
}

/// Handle to a GPU buffer that is automatically released when dropped.
#[derive(Clone)]
pub struct GPUBuffer {
    pub(crate) inner: Rc<GPUBufferData>,
}

impl std::fmt::Debug for GPUBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GPUBuffer")
            .field("raw", &(self.inner.raw.get() as usize))
            .field("size", &self.inner.size.get())
            .finish()
    }
}

impl PartialEq for GPUBuffer {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.inner) == Rc::as_ptr(&other.inner)
    }
}

impl Eq for GPUBuffer {}

impl std::hash::Hash for GPUBuffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.inner).hash(state);
    }
}

impl Default for GPUBuffer {
    fn default() -> Self {
        GPUBuffer::none()
    }
}

impl GPUBuffer {
    pub fn none() -> GPUBuffer {
        NONE_GPUBUFFER.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.get().is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUBuffer {
        self.inner.raw.get()
    }

    pub fn size(&self) -> u32 {
        self.inner.size.get()
    }

    pub fn update(&mut self, device: &Device, usage: SDL_GPUBufferUsageFlags, copy_pass: Option<&CopyPass>, offset: u32, data: &[u8]) -> Result<(), String> {
        if !self.is_valid()
        {
            if data.len()==0 { return Ok(());}
            *self = device.create_buffer(usage, data.len() as u32+offset)?;
        }
        self.upload(copy_pass, offset, data)
    }

    pub fn upload(&self, copy_pass: Option<&CopyPass>, offset: u32, data: &[u8]) -> Result<(), String> {
        let data_size = data.len() as u32;
        if data_size==0 { return Ok(())}
        let required_size = offset.saturating_add(data_size);
        let buf_size = self.inner.size.get();
        if required_size > buf_size {
            let weak = self.inner.device.clone();
            let di = weak.upgrade().ok_or("GPUBuffer::upload: device dropped")?;
            let info = gpu::SDL_GPUBufferCreateInfo {
                usage: self.inner.usage,
                size: required_size,
                props: sys::properties::SDL_PropertiesID(0),
            };
            let (new_raw, old_raw) = unsafe {
                let new_raw = gpu::SDL_CreateGPUBuffer(di.raw, &info);
                if new_raw.is_null() {
                    return Err(sdl_fail("SDL_CreateGPUBuffer"));
                }
                (new_raw, self.inner.raw.get())
            };
            let preserve_size = offset.min(buf_size);
            if preserve_size > 0 {
                let src = gpu::SDL_GPUBufferLocation { buffer: old_raw, offset: 0 };
                let dst = gpu::SDL_GPUBufferLocation { buffer: new_raw, offset: 0 };
                di.with_copy_pass(copy_pass, |pass| unsafe {
                    gpu::SDL_CopyGPUBufferToBuffer(pass, &src, &dst, preserve_size, false);
                })?;
            }
            unsafe {
                if !old_raw.is_null() {
                    gpu::SDL_ReleaseGPUBuffer(di.raw, old_raw);
                }
                self.inner.raw.set(new_raw);
                self.inner.size.set(required_size);
            }
        }
        let weak = self.inner.device.clone();
        let di = weak.upgrade().ok_or("GPUBuffer::upload: device dropped")?;
        let transfer = di.stage_upload(data, &weak)?;
        let src = gpu::SDL_GPUTransferBufferLocation { transfer_buffer: transfer.raw(), offset: 0 };
        let dst = gpu::SDL_GPUBufferRegion { buffer: self.raw(), offset, size: data_size };
        di.with_copy_pass(copy_pass, |pass| unsafe {
            gpu::SDL_UploadToGPUBuffer(pass, &src, &dst, offset == 0);
        })
    }

    pub fn download_vecu8(&self, offset: u32, size: u32) -> Result<Vec<u8>, String> {
        let buf_size = self.size();
        let size = if size == 0 { buf_size - offset } else { size };
        if offset.saturating_add(size) > buf_size {
            return Err("requested range exceeds buffer size".into());
        }
        let weak = self.inner.device.clone();
        let di = weak.upgrade().ok_or("GPUBuffer::download_raw: device dropped")?;
        let transfer = di.create_download_transfer_buffer(size, &weak)?;
        unsafe {
            let cmd = gpu::SDL_AcquireGPUCommandBuffer(di.raw);
            if cmd.is_null() {
                return Err(sdl_fail("SDL_AcquireGPUCommandBuffer"));
            }
            let pass = gpu::SDL_BeginGPUCopyPass(cmd);
            if pass.is_null() {
                gpu::SDL_CancelGPUCommandBuffer(cmd);
                return Err(sdl_fail("SDL_BeginGPUCopyPass"));
            }

            let src = gpu::SDL_GPUBufferRegion { buffer: self.raw(), offset, size };
            let dst = gpu::SDL_GPUTransferBufferLocation { transfer_buffer: transfer.raw(), offset: 0 };
            gpu::SDL_DownloadFromGPUBuffer(pass, &src, &dst);
            gpu::SDL_EndGPUCopyPass(pass);

            let fence = gpu::SDL_SubmitGPUCommandBufferAndAcquireFence(cmd);
            if fence.is_null() {
                return Err(sdl_fail("SDL_SubmitGPUCommandBufferAndAcquireFence"));
            }
            if !gpu::SDL_WaitForGPUFences(di.raw, true, &fence, 1) {
                gpu::SDL_ReleaseGPUFence(di.raw, fence);
                return Err(sdl_fail("SDL_WaitForGPUFences"));
            }
            gpu::SDL_ReleaseGPUFence(di.raw, fence);
        }
        transfer.read()
    }

    pub fn download<T: bytemuck::Pod>(&self, offset: u32, dst: &mut T) -> Result<(), String> {
        let size = std::mem::size_of::<T>() as u32;
        let data = self.download_vecu8(offset, size)?;
        *dst = *bytemuck::from_bytes::<T>(&data);
        Ok(())
    }
}

thread_local! {
    static NONE_SAMPLER: Sampler = Sampler {
        inner: Rc::new(SamplerData { raw: std::ptr::null_mut(), device: Weak::new() }),
    };
}

/// Handle to a GPU sampler that is automatically released when dropped.
#[derive(Clone)]
pub struct Sampler {
    pub(crate) inner: Rc<SamplerData>,
}

impl std::fmt::Debug for Sampler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sampler")
            .field("raw", &(self.inner.raw as usize))
            .finish()
    }
}

impl PartialEq for Sampler {
    fn eq(&self, other: &Self) -> bool {
        Rc::as_ptr(&self.inner) == Rc::as_ptr(&other.inner)
    }
}

impl Eq for Sampler {}

impl std::hash::Hash for Sampler {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.inner).hash(state);
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler::none()
    }
}

impl Sampler {
    pub fn none() -> Sampler {
        NONE_SAMPLER.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUSampler {
        self.inner.raw
    }
}

/// A texture+sampler pair for binding to a shader slot.
pub struct TextureSamplerBinding<'a> {
    pub texture: &'a Texture,
    pub sampler: &'a Sampler,
}

pub struct GPUBufferBinding<'a> {
    /// The buffer to bind.
    pub buffer: &'a GPUBuffer,
    /// The starting byte offset within the buffer.
    pub offset: u32,
}


pub struct GraphicsPipelineCreateInfo<'a> {
    /// The vertex shader used by the graphics pipeline.
    pub vertex_shader: Shader,
    /// The fragment shader used by the graphics pipeline.
    pub fragment_shader: Shader,
    /// Vertex attribute descriptions.
    pub vertex_attributes: &'a [SDL_GPUVertexAttribute],
    /// Vertex buffer descriptions.
    pub vertex_buffer_descriptions: &'a [SDL_GPUVertexBufferDescription],
    /// The primitive topology of the graphics pipeline.
    pub primitive_type: SDL_GPUPrimitiveType,
    /// The rasterizer state of the graphics pipeline.
    pub rasterizer_state: SDL_GPURasterizerState,
    /// The multisample state of the graphics pipeline.
    pub multisample_state: SDL_GPUMultisampleState,
    /// The depth-stencil state of the graphics pipeline.
    pub depth_stencil_state: SDL_GPUDepthStencilState,
    /// Color target descriptions.
    pub color_target_descriptions: &'a [SDL_GPUColorTargetDescription],
    /// The pixel format of the depth-stencil target. Ignored if has_depth_stencil_target is false.
    pub depth_stencil_format: SDL_GPUTextureFormat,
    /// Whether the pipeline uses a depth-stencil target.
    pub has_depth_stencil_target: bool,
}

pub struct ComputePipelineCreateInfo<'a> {
    /// The compute shader bytecode.
    pub code: &'a [u8],
    /// The entry point function name.
    pub entrypoint: &'a str,
    /// The format of the shader code.
    pub format: SDL_GPUShaderFormat,
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
    /// The number of threads in the X dimension of the workgroup.
    pub threadcount_x: u32,
    /// The number of threads in the Y dimension of the workgroup.
    pub threadcount_y: u32,
    /// The number of threads in the Z dimension of the workgroup.
    pub threadcount_z: u32,
}

/// A read-write storage buffer binding for a compute pass.
pub struct StorageBufferReadWriteBinding<'a> {
    pub buffer: &'a GPUBuffer,
    pub cycle: bool,
}

/// A read-write storage texture binding for a compute pass.
pub struct StorageTextureReadWriteBinding<'a> {
    pub texture: &'a Texture,
    pub mip_level: u32,
    pub layer: u32,
    pub cycle: bool,
}

pub struct CommandBuffer {
    inner: *mut gpu::SDL_GPUCommandBuffer,
    device: Device,
    submitted: bool,
    pass_active: Cell<bool>,
    swapchain_texture: RefCell<Option<Texture>>,
}

impl CommandBuffer {
    pub fn raw(&self) -> *mut gpu::SDL_GPUCommandBuffer {
        self.inner
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn acquire_swapchain_texture(
        &self,
    ) -> Result<Option<Texture>, String> {
        let window = self.device.inner.window.as_ref()
            .ok_or_else(|| String::from("Device has no window"))?;

        let mut texture: *mut gpu::SDL_GPUTexture = std::ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;

        unsafe {
            let ok = gpu::SDL_AcquireGPUSwapchainTexture(
                self.inner,
                window.raw(),
                &mut texture,
                &mut width,
                &mut height,
            );
            if !ok {
                return Err(sdl_fail("SDL_AcquireGPUSwapchainTexture"));
            }
        }

        let sc = Texture {
            inner: Rc::new(TextureData {
                raw: texture, res: (width, height), device: Weak::new(), kind: Cell::new(TextureKind::Swapchain),
            }),
        };
        *self.swapchain_texture.borrow_mut() = Some(sc.clone());
        if texture.is_null() {
            Ok(None)
        } else {
            Ok(Some(sc))
        }
    }

    pub fn wait_and_acquire_swapchain_texture(
        &self,
    ) -> Result<Texture, String> {
        let window = self.device.inner.window.as_ref()
            .ok_or_else(|| String::from("Device has no window"))?;

        let mut texture: *mut gpu::SDL_GPUTexture = std::ptr::null_mut();
        let mut width: u32 = 0;
        let mut height: u32 = 0;

        unsafe {
            let ok = gpu::SDL_WaitAndAcquireGPUSwapchainTexture(
                self.inner,
                window.raw(),
                &mut texture,
                &mut width,
                &mut height,
            );
            if !ok {
                return Err(sdl_fail("SDL_WaitAndAcquireGPUSwapchainTexture"));
            }
        }

        let sc = Texture {
            inner: Rc::new(TextureData {
                raw: texture, res: (width, height), device: Weak::new(), kind: Cell::new(TextureKind::Swapchain),
            }),
        };
        *self.swapchain_texture.borrow_mut() = Some(sc.clone());
        Ok(sc)
    }

    pub fn submit(mut self) -> Result<(), String> {
        // Mark submitted before the call — SDL consumes the command buffer
        // regardless of success/failure, so Drop must not cancel it.
        self.submitted = true;
        self.device.on_command_buffer_done();
        unsafe {
            if !gpu::SDL_SubmitGPUCommandBuffer(self.inner) {
                return Err(sdl_fail("SDL_SubmitGPUCommandBuffer"));
            }
        }
        Ok(())
    }

    /// Submit the command buffer and return a fence handle that can be waited on.
    pub fn submit_and_acquire_fence(mut self) -> Result<Fence, String> {
        self.submitted = true;
        self.device.on_command_buffer_done();
        unsafe {
            let fence_ptr = gpu::SDL_SubmitGPUCommandBufferAndAcquireFence(self.inner);
            if fence_ptr.is_null() {
                return Err(sdl_fail("SDL_SubmitGPUCommandBufferAndAcquireFence"));
            }
            let weak = Rc::downgrade(&self.device.inner);
            Ok(Fence { inner: Rc::new(FenceData { raw: fence_ptr, device: weak }) })
        }
    }
}

impl CommandBuffer {
    /// Blit from a source texture region to a destination texture region.
    /// Must not be called inside any pass.
    pub fn blit_texture(&mut self, info: &BlitInfo) {
        let raw = info.to_raw(&self.device);
        unsafe {
            gpu::SDL_BlitGPUTexture(self.inner, &raw);
        }
    }
}

impl CommandBuffer {
    pub fn begin_copy_pass<'b>(&'b self) -> Result<CopyPass<'b>, String> {
        assert!(!self.pass_active.get(), "a pass is already active on this command buffer");
        unsafe {
            let raw = gpu::SDL_BeginGPUCopyPass(self.inner);
            if raw.is_null() {
                return Err(sdl_fail("SDL_BeginGPUCopyPass"));
            }
            self.pass_active.set(true);
            Ok(CopyPass { inner: raw, pass_active: &self.pass_active })
        }
    }
    pub fn begin_render_pass<'b>(
        &'b self,
        color_targets: &[ColorTargetInfo],
        depth_stencil_target: Option<&DepthStencilTargetInfo>,
    ) -> Result<RenderPass<'b>, String> {
        assert!(!self.pass_active.get(), "a pass is already active on this command buffer");
        let raw_targets: Vec<gpu::SDL_GPUColorTargetInfo> = color_targets
            .iter()
            .map(|ct| ct.to_raw(&self.device))
            .collect();

        let raw_ds = depth_stencil_target.map(|ds| ds.to_raw(&self.device));
        let ds_ptr = raw_ds
            .as_ref()
            .map(|ds| ds as *const gpu::SDL_GPUDepthStencilTargetInfo)
            .unwrap_or(std::ptr::null());

        unsafe {
            let raw = gpu::SDL_BeginGPURenderPass(
                self.inner,
                raw_targets.as_ptr(),
                raw_targets.len() as u32,
                ds_ptr,
            );
            if raw.is_null() {
                return Err(sdl_fail("SDL_BeginGPURenderPass"));
            }
            self.pass_active.set(true);
            Ok(RenderPass { inner: raw, cmd_buf: self.inner, device: self.device.clone(), pass_active: &self.pass_active })
        }
    }

    #[allow(deprecated)]
    pub fn begin_compute_pass<'b>(
        &'b self,
        storage_texture_bindings: &[StorageTextureReadWriteBinding<'_>],
        storage_buffer_bindings: &[StorageBufferReadWriteBinding<'_>],
    ) -> Result<ComputePass<'b>, String> {
        assert!(!self.pass_active.get(), "a pass is already active on this command buffer");
        let raw_tex_bindings: Vec<gpu::SDL_GPUStorageTextureReadWriteBinding> = storage_texture_bindings
            .iter()
            .map(|b| gpu::SDL_GPUStorageTextureReadWriteBinding {
                texture: b.texture.raw(),
                mip_level: b.mip_level,
                layer: b.layer,
                cycle: b.cycle,
                padding1: 0,
                padding2: 0,
                padding3: 0,
            })
            .collect();
        let raw_buf_bindings: Vec<gpu::SDL_GPUStorageBufferReadWriteBinding> = storage_buffer_bindings
            .iter()
            .map(|b| gpu::SDL_GPUStorageBufferReadWriteBinding {
                buffer: b.buffer.raw(),
                cycle: b.cycle,
                padding1: 0,
                padding2: 0,
                padding3: 0,
            })
            .collect();
        unsafe {
            let raw = gpu::SDL_BeginGPUComputePass(
                self.inner,
                if raw_tex_bindings.is_empty() { std::ptr::null() } else { raw_tex_bindings.as_ptr() },
                raw_tex_bindings.len() as u32,
                if raw_buf_bindings.is_empty() { std::ptr::null() } else { raw_buf_bindings.as_ptr() },
                raw_buf_bindings.len() as u32,
            );
            if raw.is_null() {
                return Err(sdl_fail("SDL_BeginGPUComputePass"));
            }
            self.pass_active.set(true);
            Ok(ComputePass { inner: raw, cmd_buf: self.inner, pass_active: &self.pass_active })
        }
    }
}

pub struct RenderPass<'b> {
    inner: *mut gpu::SDL_GPURenderPass,
    cmd_buf: *mut gpu::SDL_GPUCommandBuffer,
    pub device: Device,
    pass_active: &'b Cell<bool>,
}

impl RenderPass<'_> {
    pub fn bind_vertex_buffers(&self, first_slot: u32, bindings: &[GPUBufferBinding<'_>]) {
        let raw_bindings: Vec<gpu::SDL_GPUBufferBinding> = bindings
            .iter()
            .map(|b| gpu::SDL_GPUBufferBinding {
                buffer: b.buffer.raw(),
                offset: b.offset,
            })
            .collect();
        unsafe {
            gpu::SDL_BindGPUVertexBuffers(
                self.inner,
                first_slot,
                raw_bindings.as_ptr(),
                raw_bindings.len() as u32,
            );
        }
    }

    pub fn bind_graphics_pipeline(&self, pipeline: &GraphicsPipeline) {
        unsafe {
            gpu::SDL_BindGPUGraphicsPipeline(
                self.inner,
                pipeline.raw(),
            );
        }
    }

    pub fn draw_primitives(&self, num_vertices: u32, num_instances: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            gpu::SDL_DrawGPUPrimitives(self.inner, num_vertices, num_instances, first_vertex, first_instance);
        }
    }

    pub fn draw_indexed_primitives(&self, num_indices: u32, num_instances: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            gpu::SDL_DrawGPUIndexedPrimitives(self.inner, num_indices, num_instances, first_index, vertex_offset, first_instance);
        }
    }

    pub fn draw_primitives_indirect(&self, buffer: &GPUBuffer, offset: u32, draw_count: u32) {
        unsafe {
            gpu::SDL_DrawGPUPrimitivesIndirect(self.inner, buffer.raw(), offset, draw_count);
        }
    }

    pub fn draw_indexed_primitives_indirect(&self, buffer: &GPUBuffer, offset: u32, draw_count: u32) {
        unsafe {
            gpu::SDL_DrawGPUIndexedPrimitivesIndirect(self.inner, buffer.raw(), offset, draw_count);
        }
    }

    pub fn bind_fragment_samplers(&self, first_slot: u32, bindings: &[TextureSamplerBinding<'_>]) {
        let raw_bindings: Vec<gpu::SDL_GPUTextureSamplerBinding> = bindings
            .iter()
            .map(|b| gpu::SDL_GPUTextureSamplerBinding {
                texture: b.texture.raw(),
                sampler: b.sampler.raw(),
            })
            .collect();
        unsafe {
            gpu::SDL_BindGPUFragmentSamplers(
                self.inner,
                first_slot,
                raw_bindings.as_ptr(),
                raw_bindings.len() as u32,
            );
        }
    }

    pub fn push_vertex_uniform_data(&self, slot_index: u32, data: &[u8]) {
        unsafe {
            gpu::SDL_PushGPUVertexUniformData(
                self.cmd_buf,
                slot_index,
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as u32,
            );
        }
    }

    pub fn push_fragment_uniform_data(&self, slot_index: u32, data: &[u8]) {
        unsafe {
            gpu::SDL_PushGPUFragmentUniformData(
                self.cmd_buf,
                slot_index,
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as u32,
            );
        }
    }

    pub fn bind_index_buffer(&self, binding: &GPUBufferBinding<'_>, index_element_size: SDL_GPUIndexElementSize) {
        assert!(binding.buffer.is_valid());
        let raw = gpu::SDL_GPUBufferBinding {
            buffer: binding.buffer.raw(),
            offset: binding.offset,
        };
        unsafe {
            gpu::SDL_BindGPUIndexBuffer(self.inner, &raw, index_element_size);
        }
    }

    pub fn set_viewport(&self, viewport: &SDL_GPUViewport) {
        unsafe {
            gpu::SDL_SetGPUViewport(self.inner, viewport);
        }
    }

    pub fn set_scissor(&self, rect: &SDL_Rect) {
        unsafe {
            gpu::SDL_SetGPUScissor(self.inner, rect);
        }
    }

    pub fn set_stencil_reference(&self, reference: u8) {
        unsafe {
            gpu::SDL_SetGPUStencilReference(self.inner, reference);
        }
    }

    pub fn set_blend_constants(&self, blend_constants: SDL_FColor) {
        unsafe {
            gpu::SDL_SetGPUBlendConstants(self.inner, blend_constants);
        }
    }

    pub fn bind_fragment_storage_textures(&self, first_slot: u32, textures: &[&Texture]) {
        let raw: Vec<*mut gpu::SDL_GPUTexture> = textures
            .iter()
            .map(|t| t.raw())
            .collect();
        unsafe {
            gpu::SDL_BindGPUFragmentStorageTextures(
                self.inner,
                first_slot,
                raw.as_ptr(),
                raw.len() as u32,
            );
        }
    }

    pub fn bind_fragment_storage_buffers(&self, first_slot: u32, buffers: &[&GPUBuffer]) {
        let raw: Vec<*mut gpu::SDL_GPUBuffer> = buffers
            .iter()
            .map(|b| b.raw())
            .collect();
        unsafe {
            gpu::SDL_BindGPUFragmentStorageBuffers(
                self.inner,
                first_slot,
                raw.as_ptr(),
                raw.len() as u32,
            );
        }
    }

    pub fn bind_vertex_storage_buffers(&self, first_slot: u32, buffers: &[&GPUBuffer]) {
        let raw: Vec<*mut gpu::SDL_GPUBuffer> = buffers
            .iter()
            .map(|b| b.raw())
            .collect();
        unsafe {
            gpu::SDL_BindGPUVertexStorageBuffers(
                self.inner,
                first_slot,
                raw.as_ptr(),
                raw.len() as u32,
            );
        }
    }
}

impl Drop for RenderPass<'_> {
    fn drop(&mut self) {
        unsafe {
            gpu::SDL_EndGPURenderPass(self.inner);
        }
        self.pass_active.set(false);
    }
}

pub struct CopyPass<'b> {
    pub(crate) inner: *mut gpu::SDL_GPUCopyPass,
    pass_active: &'b Cell<bool>,
}

impl CopyPass<'_> {
    pub fn copy_buffer_to_buffer(
        &self,
        source: &GPUBuffer,
        source_offset: u32,
        destination: &GPUBuffer,
        destination_offset: u32,
        size: u32,
        cycle: bool,
    ) {
        let src = gpu::SDL_GPUBufferLocation {
            buffer: source.raw(),
            offset: source_offset,
        };
        let dst = gpu::SDL_GPUBufferLocation {
            buffer: destination.raw(),
            offset: destination_offset,
        };
        unsafe {
            gpu::SDL_CopyGPUBufferToBuffer(self.inner, &src, &dst, size, cycle);
        }
    }
}

impl Drop for CopyPass<'_> {
    fn drop(&mut self) {
        unsafe {
            gpu::SDL_EndGPUCopyPass(self.inner);
        }
        self.pass_active.set(false);
    }
}

pub struct ComputePass<'b> {
    inner: *mut gpu::SDL_GPUComputePass,
    cmd_buf: *mut gpu::SDL_GPUCommandBuffer,
    pass_active: &'b Cell<bool>,
}

impl ComputePass<'_> {
    pub fn bind_compute_pipeline(&self, pipeline: &ComputePipeline) {
        unsafe {
            gpu::SDL_BindGPUComputePipeline(
                self.inner,
                pipeline.raw(),
            );
        }
    }

    pub fn bind_storage_textures(&self, first_slot: u32, textures: &[&Texture]) {
        let raw: Vec<*mut gpu::SDL_GPUTexture> = textures
            .iter()
            .map(|t| t.raw())
            .collect();
        unsafe {
            gpu::SDL_BindGPUComputeStorageTextures(
                self.inner,
                first_slot,
                raw.as_ptr(),
                raw.len() as u32,
            );
        }
    }

    pub fn bind_storage_buffers(&self, first_slot: u32, buffers: &[&GPUBuffer]) {
        let raw: Vec<*mut gpu::SDL_GPUBuffer> = buffers
            .iter()
            .map(|b| b.raw())
            .collect();
        unsafe {
            gpu::SDL_BindGPUComputeStorageBuffers(
                self.inner,
                first_slot,
                raw.as_ptr(),
                raw.len() as u32,
            );
        }
    }

    pub fn bind_samplers(&self, first_slot: u32, bindings: &[TextureSamplerBinding<'_>]) {
        let raw_bindings: Vec<gpu::SDL_GPUTextureSamplerBinding> = bindings
            .iter()
            .map(|b| gpu::SDL_GPUTextureSamplerBinding {
                texture: b.texture.raw(),
                sampler: b.sampler.raw(),
            })
            .collect();
        unsafe {
            gpu::SDL_BindGPUComputeSamplers(
                self.inner,
                first_slot,
                raw_bindings.as_ptr(),
                raw_bindings.len() as u32,
            );
        }
    }

    pub fn push_compute_uniform_data(&self, slot_index: u32, data: &[u8]) {
        unsafe {
            gpu::SDL_PushGPUComputeUniformData(
                self.cmd_buf,
                slot_index,
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as u32,
            );
        }
    }

    pub fn dispatch(&self, groupcount_x: u32, groupcount_y: u32, groupcount_z: u32) {
        unsafe {
            gpu::SDL_DispatchGPUCompute(self.inner, groupcount_x, groupcount_y, groupcount_z);
        }
    }

    pub fn dispatch_indirect(&self, buffer: &GPUBuffer, offset: u32) {
        unsafe {
            gpu::SDL_DispatchGPUComputeIndirect(self.inner, buffer.raw(), offset);
        }
    }
}

impl Drop for ComputePass<'_> {
    fn drop(&mut self) {
        unsafe {
            gpu::SDL_EndGPUComputePass(self.inner);
        }
        self.pass_active.set(false);
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        if let Some(ref sc) = *self.swapchain_texture.borrow() {
            sc.inner.kind.set(TextureKind::None);
        }
        if !self.submitted {
            unsafe {
                gpu::SDL_CancelGPUCommandBuffer(self.inner);
            }
            self.device.on_command_buffer_done();
        }
    }
}

thread_local! {
    static NONE_FENCE: Fence = Fence {
        inner: Rc::new(FenceData { raw: std::ptr::null_mut(), device: Weak::new() }),
    };
}

/// Handle to a fence returned by `submit_and_acquire_fence`.
#[derive(Clone)]
pub struct Fence {
    pub(crate) inner: Rc<FenceData>,
}

impl std::fmt::Debug for Fence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fence")
            .field("raw", &(self.inner.raw as usize))
            .finish()
    }
}

impl Default for Fence {
    fn default() -> Self {
        Fence::none()
    }
}

impl Fence {
    pub fn none() -> Fence {
        NONE_FENCE.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }
}

pub(crate) struct TransferBufferData {
    pub(crate) raw: *mut gpu::SDL_GPUTransferBuffer,
    size: u32,
    device: Weak<DeviceInner>,
}

impl Drop for TransferBufferData {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }
        if let Some(di) = self.device.upgrade() {
            unsafe { gpu::SDL_ReleaseGPUTransferBuffer(di.raw, self.raw); }
        }
    }
}

thread_local! {
    static NONE_TRANSFER_BUFFER: GPUTransferBuffer = GPUTransferBuffer {
        inner: Rc::new(TransferBufferData {
            raw: std::ptr::null_mut(), size: 0, device: Weak::new(),
        }),
    };
}

/// A GPU transfer buffer for upload/download staging. Automatically released on drop.
#[derive(Clone)]
pub struct GPUTransferBuffer {
    pub(crate) inner: Rc<TransferBufferData>,
}

impl std::fmt::Debug for GPUTransferBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GPUTransferBuffer")
            .field("raw", &(self.inner.raw as usize))
            .field("size", &self.inner.size)
            .finish()
    }
}

impl Default for GPUTransferBuffer {
    fn default() -> Self {
        GPUTransferBuffer::none()
    }
}

impl GPUTransferBuffer {
    pub fn none() -> GPUTransferBuffer {
        NONE_TRANSFER_BUFFER.with(|s| s.clone())
    }

    pub fn is_valid(&self) -> bool {
        !self.inner.raw.is_null()
    }

    pub fn raw(&self) -> *mut gpu::SDL_GPUTransferBuffer {
        self.inner.raw
    }

    pub fn size(&self) -> u32 {
        self.inner.size
    }

    /// Map, write data, and unmap the transfer buffer.
    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        let size = self.size();
        if data.len() as u32 > size {
            return Err("data exceeds transfer buffer size".into());
        }
        let di = self.inner.device.upgrade().ok_or("GPUTransferBuffer::write: device dropped")?;
        unsafe {
            let ptr = gpu::SDL_MapGPUTransferBuffer(di.raw, self.raw(), true);
            if ptr.is_null() {
                return Err(sdl_fail("SDL_MapGPUTransferBuffer"));
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            gpu::SDL_UnmapGPUTransferBuffer(di.raw, self.raw());
        }
        Ok(())
    }

    /// Map, read, and unmap the transfer buffer.
    pub fn read(&self) -> Result<Vec<u8>, String> {
        let size = self.size();
        let di = self.inner.device.upgrade().ok_or("GPUTransferBuffer::read: device dropped")?;
        unsafe {
            let ptr = gpu::SDL_MapGPUTransferBuffer(di.raw, self.raw(), false);
            if ptr.is_null() {
                return Err(sdl_fail("SDL_MapGPUTransferBuffer"));
            }
            let mut data = vec![0u8; size as usize];
            std::ptr::copy_nonoverlapping(ptr as *const u8, data.as_mut_ptr(), size as usize);
            gpu::SDL_UnmapGPUTransferBuffer(di.raw, self.raw());
            Ok(data)
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) != 1 {
            return;
        }
        let r = self.inner.raw;
        unsafe {
            *self.inner.upload_transfer_buffer.borrow_mut() = None;
            self.inner.pending_transfer_buffers.borrow_mut().clear();
            if let Some(window) = &self.inner.window
            {
                gpu::SDL_ReleaseWindowFromGPUDevice(r, window.raw());
            }
            gpu::SDL_DestroyGPUDevice(r);
        }
    }
}

