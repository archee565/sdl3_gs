use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, Ordering};

use sdl3_sys as sys;
use sys::*;

pub use gpu::SDL_GPUShaderFormat;
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
pub use gpu::SDL_GPUFilter;
pub use gpu::SDL_GPUTextureUsageFlags;
pub use gpu::SDL_GPUTextureType;
pub use sys::pixels::SDL_FColor;
pub use sys::surface::SDL_FlipMode;

use crate::slot_map::SlotMapRefCell;

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
    pub(crate) fn to_raw(&self, device: &Device) -> gpu::SDL_GPUColorTargetInfo {
        gpu::SDL_GPUColorTargetInfo {
            texture: device.texture_raw(self.texture),
            mip_level: self.mip_level,
            layer_or_depth_plane: self.layer_or_depth_plane,
            clear_color: self.clear_color,
            load_op: self.load_op,
            store_op: self.store_op,
            resolve_texture: self.resolve_texture
                .map(|t| device.texture_raw(t))
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

    pub(crate) fn to_raw(&self, device: &Device) -> gpu::SDL_GPUDepthStencilTargetInfo {
        gpu::SDL_GPUDepthStencilTargetInfo {
            texture: device.texture_raw(self.texture),
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

    pub(crate) fn to_raw(&self, device: &Device) -> gpu::SDL_GPUBlitRegion {
        gpu::SDL_GPUBlitRegion {
            texture: device.texture_raw(self.texture),
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

pub struct Device
{
    inner: *mut gpu::SDL_GPUDevice,
    window : Option<crate::window::Window>,
    textures: SlotMapRefCell<TextureSlot>,
    shaders: SlotMapRefCell<ShaderSlot>,
    graphics_pipelines: SlotMapRefCell<GraphicsPipelineSlot>,
    buffers: SlotMapRefCell<BufferSlot>,
    swapchain: Cell<(*mut gpu::SDL_GPUTexture, u32, u32)>,
    upload_transfer_buffer: Cell<(*mut gpu::SDL_GPUTransferBuffer, u32)>,
    cmd_buf_count: AtomicU32,
    pending_transfer_buffers: RefCell<Vec<*mut gpu::SDL_GPUTransferBuffer>>,
}

impl Device {
    pub fn new(format : gpu::SDL_GPUShaderFormat, window : Option<crate::window::Window>) -> Result<Self,&'static str>
    {
        unsafe {
            let sys_device = gpu::SDL_CreateGPUDevice(
                format,
                true,
                std::ptr::null(),
            );

            if sys_device.is_null()
            {
                return Err("SDL CreateGPUDevice failed.");
            }

            if let Some(window) = &window
            {
                gpu::SDL_ClaimWindowForGPUDevice(sys_device,window.raw());
            }
            
            Ok(Device {
                inner: sys_device,
                window,
                textures: SlotMapRefCell::new(),
                shaders: SlotMapRefCell::new(),
                graphics_pipelines: SlotMapRefCell::new(),
                buffers: SlotMapRefCell::new(),
                swapchain: Cell::new((std::ptr::null_mut(), 0, 0)),
                upload_transfer_buffer: Cell::new((std::ptr::null_mut(), 0)),
                cmd_buf_count: AtomicU32::new(0),
                pending_transfer_buffers: RefCell::new(Vec::new()),
            })
        }
        
    }

    pub fn create_texture(&self, info: &gpu::SDL_GPUTextureCreateInfo) -> Result<Texture, &'static str> {
        unsafe {
            let raw = gpu::SDL_CreateGPUTexture(self.inner, info);
            if raw.is_null() {
                return Err("SDL_CreateGPUTexture failed");
            }
            let slot = TextureSlot {
                inner: raw,
                res: (info.width, info.height),
            };
            let idx = self.textures.insert(slot);
            Ok(Texture(idx))
        }
    }

    pub fn destroy_texture(&self, handle: Texture) {
        let slot = self.textures.remove(handle.0);
        unsafe {
            gpu::SDL_ReleaseGPUTexture(self.inner, slot.inner);
        }
    }

    pub(crate) fn texture_raw(&self, handle: Texture) -> *mut gpu::SDL_GPUTexture {
        if handle == Texture::SWAPCHAIN {
            let (ptr, _, _) = self.swapchain.get();
            assert!(!ptr.is_null(), "no swapchain texture acquired");
            return ptr;
        }
        self.textures.with(handle.0, |slot| slot.inner)
    }

    pub fn texture_res(&self, handle: Texture) -> (u32, u32) {
        if handle == Texture::SWAPCHAIN {
            let (ptr, w, h) = self.swapchain.get();
            assert!(!ptr.is_null(), "no swapchain texture acquired");
            return (w, h);
        }
        self.textures.with(handle.0, |slot| slot.res)
    }

    pub fn create_shader(&self, info: &ShaderCreateInfo) -> Result<Shader, &'static str> {
        let entrypoint = std::ffi::CString::new(info.entrypoint)
            .map_err(|_| "entrypoint contains interior nul byte")?;
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
            let raw = gpu::SDL_CreateGPUShader(self.inner, &raw_info);
            if raw.is_null() {
                return Err("SDL_CreateGPUShader failed");
            }
            let idx = self.shaders.insert(ShaderSlot { inner: raw });
            Ok(Shader(idx))
        }
    }

    pub fn destroy_shader(&self, handle: Shader) {
        let slot = self.shaders.remove(handle.0);
        unsafe {
            gpu::SDL_ReleaseGPUShader(self.inner, slot.inner);
        }
    }

    #[allow(deprecated)]
    pub fn create_graphics_pipeline(&self, info: &GraphicsPipelineCreateInfo) -> Result<GraphicsPipeline, &'static str> {
        let vertex_shader_raw = self.shaders.with(info.vertex_shader.0, |s| s.inner);
        let fragment_shader_raw = self.shaders.with(info.fragment_shader.0, |s| s.inner);
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
            let raw = gpu::SDL_CreateGPUGraphicsPipeline(self.inner, &raw_info);
            if raw.is_null() {
                return Err("SDL_CreateGPUGraphicsPipeline failed");
            }
            let idx = self.graphics_pipelines.insert(GraphicsPipelineSlot { inner: raw });
            Ok(GraphicsPipeline(idx))
        }
    }

    pub fn destroy_graphics_pipeline(&self, handle: GraphicsPipeline) {
        let slot = self.graphics_pipelines.remove(handle.0);
        unsafe {
            gpu::SDL_ReleaseGPUGraphicsPipeline(self.inner, slot.inner);
        }
    }

    pub fn create_buffer(&self, usage: SDL_GPUBufferUsageFlags, size: u32) -> Result<GPUBuffer, &'static str> {
        let info = gpu::SDL_GPUBufferCreateInfo {
            usage,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUBuffer(self.inner, &info);
            if raw.is_null() {
                return Err("SDL_CreateGPUBuffer failed");
            }
            let idx = self.buffers.insert(BufferSlot { inner: raw });
            Ok(GPUBuffer(idx))
        }
    }

    pub fn destroy_buffer(&self, handle: GPUBuffer) {
        let slot = self.buffers.remove(handle.0);
        unsafe {
            gpu::SDL_ReleaseGPUBuffer(self.inner, slot.inner);
        }
    }

    pub(crate) fn buffer_raw(&self, handle: GPUBuffer) -> *mut gpu::SDL_GPUBuffer {
        self.buffers.with(handle.0, |slot| slot.inner)
    }

    /// Ensure the internal upload transfer buffer is at least `size` bytes.
    /// Grows by releasing the old one and creating a new one if needed.
    fn ensure_upload_transfer_buffer(&self, size: u32) -> Result<*mut gpu::SDL_GPUTransferBuffer, &'static str> {
        let (current, current_size) = self.upload_transfer_buffer.get();
        if !current.is_null() && current_size >= size {
            return Ok(current);
        }
        // Defer release of the old buffer until no command buffers are in flight.
        if !current.is_null() {
            self.pending_transfer_buffers.borrow_mut().push(current);
        }
        let tb_info = gpu::SDL_GPUTransferBufferCreateInfo {
            usage: gpu::SDL_GPUTransferBufferUsage::UPLOAD,
            size,
            props: sys::properties::SDL_PropertiesID(0),
        };
        unsafe {
            let raw = gpu::SDL_CreateGPUTransferBuffer(self.inner, &tb_info);
            if raw.is_null() {
                self.upload_transfer_buffer.set((std::ptr::null_mut(), 0));
                return Err("SDL_CreateGPUTransferBuffer failed");
            }
            self.upload_transfer_buffer.set((raw, size));
            Ok(raw)
        }
    }

    /// Upload data from a byte slice into a GPU buffer.
    /// Uses an internal transfer buffer with auto-cycling to avoid stalls.
    /// If `copy_pass` is provided, the upload is recorded into it. Otherwise, a
    /// temporary command buffer and copy pass are created and submitted.
    pub fn upload_to_buffer(&self, copy_pass: Option<&CopyPass>, buffer: GPUBuffer, offset: u32, data: &[u8]) -> Result<(), &'static str> {
        let size = data.len() as u32;
        let transfer = self.ensure_upload_transfer_buffer(size)?;
        unsafe {
            let ptr = gpu::SDL_MapGPUTransferBuffer(self.inner, transfer, true);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            gpu::SDL_UnmapGPUTransferBuffer(self.inner, transfer);

            let src = gpu::SDL_GPUTransferBufferLocation {
                transfer_buffer: transfer,
                offset: 0,
            };
            let dst = gpu::SDL_GPUBufferRegion {
                buffer: self.buffer_raw(buffer),
                offset,
                size,
            };

            if let Some(pass) = copy_pass {
                gpu::SDL_UploadToGPUBuffer(pass.inner, &src, &dst, true);
            } else {
                let cmd = gpu::SDL_AcquireGPUCommandBuffer(self.inner);
                if cmd.is_null() {
                    return Err("SDL_AcquireGPUCommandBuffer failed");
                }
                let tmp_pass = gpu::SDL_BeginGPUCopyPass(cmd);
                if tmp_pass.is_null() {
                    gpu::SDL_CancelGPUCommandBuffer(cmd);
                    return Err("SDL_BeginGPUCopyPass failed");
                }
                gpu::SDL_UploadToGPUBuffer(tmp_pass, &src, &dst, true);
                gpu::SDL_EndGPUCopyPass(tmp_pass);
                if !gpu::SDL_SubmitGPUCommandBuffer(cmd) {
                    return Err("SDL_SubmitGPUCommandBuffer failed");
                }
            }
        }
        Ok(())
    }

    pub fn get_swapchain_texture_format(&self) -> SDL_GPUTextureFormat {
        let window = self.window.as_ref().expect("Device has no window");
        unsafe { gpu::SDL_GetGPUSwapchainTextureFormat(self.inner, window.raw()) }
    }

    pub fn get_shader_formats(&self) -> SDL_GPUShaderFormat {
        unsafe { gpu::SDL_GetGPUShaderFormats(self.inner) }
    }

    pub fn acquire_command_buffer(&self) -> Result<CommandBuffer<'_>, &'static str> {
        unsafe {
            let raw = gpu::SDL_AcquireGPUCommandBuffer(self.inner);
            if raw.is_null() {
                return Err("SDL_AcquireGPUCommandBuffer failed");
            }
            self.cmd_buf_count.fetch_add(1, Ordering::Relaxed);
            Ok(CommandBuffer { inner: raw, device: self, submitted: false })
        }
    }

    /// Called when a command buffer is submitted or cancelled.
    /// When no command buffers remain in flight, releases all deferred transfer buffers.
    fn on_command_buffer_done(&self) {
        let prev = self.cmd_buf_count.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(prev > 0, "command buffer count underflow");
        if prev == 1 {
            let mut pending = self.pending_transfer_buffers.borrow_mut();
            for tb in pending.drain(..) {
                unsafe { gpu::SDL_ReleaseGPUTransferBuffer(self.inner, tb); }
            }
        }
    }
}

struct TextureSlot {
    inner: *mut gpu::SDL_GPUTexture,
    res: (u32, u32),
}

struct ShaderSlot {
    inner: *mut gpu::SDL_GPUShader,
}

struct GraphicsPipelineSlot {
    inner: *mut gpu::SDL_GPUGraphicsPipeline,
}

struct BufferSlot {
    inner: *mut gpu::SDL_GPUBuffer,
}

/// Handle to a texture stored in a `Device`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Texture(pub i32);

/// Handle to a shader stored in a `Device`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shader(pub i32);

/// Handle to a graphics pipeline stored in a `Device`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphicsPipeline(pub i32);

/// Handle to a GPU buffer stored in a `Device`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GPUBuffer(pub i32);

pub struct GPUBufferBinding {
    /// The buffer to bind.
    pub buffer: GPUBuffer,
    /// The starting byte offset within the buffer.
    pub offset: u32,
}

impl Texture {
    /// Reserved handle for the current swapchain texture.
    pub const SWAPCHAIN: Texture = Texture(-7777);
}


pub struct GraphicsPipelineCreateInfo {
    /// The vertex shader used by the graphics pipeline.
    pub vertex_shader: Shader,
    /// The fragment shader used by the graphics pipeline.
    pub fragment_shader: Shader,
    /// Vertex attribute descriptions.
    pub vertex_attributes: Vec<SDL_GPUVertexAttribute>,
    /// Vertex buffer descriptions.
    pub vertex_buffer_descriptions: Vec<SDL_GPUVertexBufferDescription>,
    /// The primitive topology of the graphics pipeline.
    pub primitive_type: SDL_GPUPrimitiveType,
    /// The rasterizer state of the graphics pipeline.
    pub rasterizer_state: gpu::SDL_GPURasterizerState,
    /// The multisample state of the graphics pipeline.
    pub multisample_state: gpu::SDL_GPUMultisampleState,
    /// The depth-stencil state of the graphics pipeline.
    pub depth_stencil_state: gpu::SDL_GPUDepthStencilState,
    /// Color target descriptions.
    pub color_target_descriptions: Vec<gpu::SDL_GPUColorTargetDescription>,
    /// The pixel format of the depth-stencil target. Ignored if has_depth_stencil_target is false.
    pub depth_stencil_format: SDL_GPUTextureFormat,
    /// Whether the pipeline uses a depth-stencil target.
    pub has_depth_stencil_target: bool,
}

pub struct CommandBuffer<'a> {
    inner: *mut gpu::SDL_GPUCommandBuffer,
    device: &'a Device,
    submitted: bool,
}

impl<'a> CommandBuffer<'a> {
    pub fn raw(&self) -> *mut gpu::SDL_GPUCommandBuffer {
        self.inner
    }

    pub fn device(&self) -> &Device {
        self.device
    }

    pub fn acquire_swapchain_texture(
        &self,
    ) -> Result<Option<Texture>, &'static str> {
        let window = self.device.window.as_ref()
            .ok_or("Device has no window")?;

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
                return Err("SDL_AcquireGPUSwapchainTexture failed");
            }
        }

        self.device.swapchain.set((texture, width, height));
        if texture.is_null() {
            Ok(None)
        } else {
            Ok(Some(Texture::SWAPCHAIN))
        }
    }

    pub fn submit(mut self) -> Result<(), &'static str> {
        // Mark submitted before the call — SDL consumes the command buffer
        // regardless of success/failure, so Drop must not cancel it.
        self.submitted = true;
        self.device.on_command_buffer_done();
        unsafe {
            if !gpu::SDL_SubmitGPUCommandBuffer(self.inner) {
                return Err("SDL_SubmitGPUCommandBuffer failed");
            }
        }
        Ok(())
    }
}

impl<'a> CommandBuffer<'a> {
    /// Blit from a source texture region to a destination texture region.
    /// Must not be called inside any pass.
    pub fn blit_texture(&mut self, info: &BlitInfo) {
        let raw = info.to_raw(self.device);
        unsafe {
            gpu::SDL_BlitGPUTexture(self.inner, &raw);
        }
    }
}

impl<'a> CommandBuffer<'a> {
    pub fn begin_copy_pass<'b>(&'b mut self) -> Result<CopyPass<'b>, &'static str> {
        unsafe {
            let raw = gpu::SDL_BeginGPUCopyPass(self.inner);
            if raw.is_null() {
                return Err("SDL_BeginGPUCopyPass failed");
            }
            Ok(CopyPass { inner: raw, _marker: std::marker::PhantomData })
        }
    }
    pub fn begin_render_pass<'b>(
        &'b mut self,
        color_targets: &[ColorTargetInfo],
        depth_stencil_target: Option<&DepthStencilTargetInfo>,
    ) -> Result<RenderPass<'b>, &'static str> {
        let raw_targets: Vec<gpu::SDL_GPUColorTargetInfo> = color_targets
            .iter()
            .map(|ct| ct.to_raw(self.device))
            .collect();

        let raw_ds = depth_stencil_target.map(|ds| ds.to_raw(self.device));
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
                return Err("SDL_BeginGPURenderPass failed");
            }
            Ok(RenderPass { inner: raw, device: self.device })
        }
    }
}

pub struct RenderPass<'b> {
    inner: *mut gpu::SDL_GPURenderPass,
    device: &'b Device,
}

impl RenderPass<'_> {
    pub fn bind_vertex_buffers(&self, first_slot: u32, bindings: &[GPUBufferBinding]) {
        let raw_bindings: Vec<gpu::SDL_GPUBufferBinding> = bindings
            .iter()
            .map(|b| gpu::SDL_GPUBufferBinding {
                buffer: self.device.buffer_raw(b.buffer),
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

    pub fn bind_graphics_pipeline(&self, pipeline: GraphicsPipeline) {
        unsafe {
            gpu::SDL_BindGPUGraphicsPipeline(
                self.inner,
                self.device.graphics_pipelines.with(pipeline.0, |slot| slot.inner),
            );
        }
    }

    pub fn draw_primitives(&self, num_vertices: u32, num_instances: u32, first_vertex: u32, first_instance: u32) {
        unsafe {
            gpu::SDL_DrawGPUPrimitives(self.inner, num_vertices, num_instances, first_vertex, first_instance);
        }
    }

    pub fn bind_index_buffer(&self, binding: &GPUBufferBinding, index_element_size: SDL_GPUIndexElementSize) {
        let raw = gpu::SDL_GPUBufferBinding {
            buffer: self.device.buffer_raw(binding.buffer),
            offset: binding.offset,
        };
        unsafe {
            gpu::SDL_BindGPUIndexBuffer(self.inner, &raw, index_element_size);
        }
    }
}

impl Drop for RenderPass<'_> {
    fn drop(&mut self) {
        unsafe {
            gpu::SDL_EndGPURenderPass(self.inner);
        }
    }
}

pub struct CopyPass<'b> {
    pub(crate) inner: *mut gpu::SDL_GPUCopyPass,
    _marker: std::marker::PhantomData<&'b mut CommandBuffer<'b>>,
}

impl Drop for CopyPass<'_> {
    fn drop(&mut self) {
        unsafe {
            gpu::SDL_EndGPUCopyPass(self.inner);
        }
    }
}

impl Drop for CommandBuffer<'_> {
    fn drop(&mut self) {
        self.device.swapchain.set((std::ptr::null_mut(), 0, 0));
        if !self.submitted {
            unsafe {
                gpu::SDL_CancelGPUCommandBuffer(self.inner);
            }
            self.device.on_command_buffer_done();
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let (tb, _) = self.upload_transfer_buffer.get();
            if !tb.is_null() {
                gpu::SDL_ReleaseGPUTransferBuffer(self.inner, tb);
            }
            for pending_tb in self.pending_transfer_buffers.borrow().iter() {
                gpu::SDL_ReleaseGPUTransferBuffer(self.inner, *pending_tb);
            }
            self.buffers.for_each(|_, slot| {
                gpu::SDL_ReleaseGPUBuffer(self.inner, slot.inner);
            });
            self.graphics_pipelines.for_each(|_, slot| {
                gpu::SDL_ReleaseGPUGraphicsPipeline(self.inner, slot.inner);
            });
            self.shaders.for_each(|_, slot| {
                gpu::SDL_ReleaseGPUShader(self.inner, slot.inner);
            });
            self.textures.for_each(|_, slot| {
                gpu::SDL_ReleaseGPUTexture(self.inner, slot.inner);
            });
            if let Some(window) = &self.window
            {
                gpu::SDL_ReleaseWindowFromGPUDevice(self.inner, window.raw());
            }
            gpu::SDL_DestroyGPUDevice(self.inner);
        }
    }
}

