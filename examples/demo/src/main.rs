#![allow(unused)]
use sdl3_gs::callbacks::App;
use sdl3_gs::device::*;
use sdl3_gs::event::{Event, WindowEventKind, SDL_Scancode};
use sdl3_gs::sys::gpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

const QUAD_VERTICES: [Vertex; 4] = [
    Vertex { pos: [-1.0, -1.0], uv: [0.0, 0.0] },
    Vertex { pos: [ 1.0, -1.0], uv: [1.0, 0.0] },
    Vertex { pos: [ 1.0,  1.0], uv: [1.0, 1.0] },
    Vertex { pos: [-1.0,  1.0], uv: [0.0, 1.0] },
];

const QUAD_INDICES: [u16; 3] = [0, 1, 2];//, 2, 3, 0];

const SAMPLE_COUNT: SDL_GPUSampleCount = SDL_GPUSampleCount::_4;

fn generate_checkerboard(width: u32, height: u32, cell_size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / cell_size) + (y / cell_size)).is_multiple_of(2);
            let c = if is_white { 255u8 } else { 64u8 };
            pixels.extend_from_slice(&[c, c, c, 255]);
        }
    }
    pixels
}

struct MsaaTargets {
    msaa: Texture,
    resolve: Texture,
    width: u32,
    height: u32,
}

impl MsaaTargets {
    fn create(device: &Device, width: u32, height: u32, format: SDL_GPUTextureFormat) -> Self {
        let msaa = device.create_texture(&SDL_GPUTextureCreateInfo {
            r#type: SDL_GPUTextureType::_2D,
            format,
            usage: SDL_GPUTextureUsageFlags::COLOR_TARGET,
            width,
            height,
            layer_count_or_depth: 1,
            num_levels: 1,
            sample_count: SAMPLE_COUNT,
            props: sdl3_gs::sys::properties::SDL_PropertiesID(0),
        }).expect("Failed to create MSAA texture");

        let resolve = device.create_texture(&SDL_GPUTextureCreateInfo {
            r#type: SDL_GPUTextureType::_2D,
            format,
            usage: SDL_GPUTextureUsageFlags::COLOR_TARGET | SDL_GPUTextureUsageFlags::SAMPLER,
            width,
            height,
            layer_count_or_depth: 1,
            num_levels: 1,
            sample_count: SDL_GPUSampleCount::_1,
            props: sdl3_gs::sys::properties::SDL_PropertiesID(0),
        }).expect("Failed to create resolve texture");

        Self { msaa, resolve, width, height }
    }

    fn destroy(&mut self, device: &Device) {
        self.msaa.destroy(device);
        self.resolve.destroy(device);
    }
}

struct Renderer {
    pipeline: GraphicsPipeline,
    vertex_buffer: GPUBuffer,
    index_buffer: GPUBuffer,
    checkerboard_texture: Texture,
    sampler: Sampler,
    targets: MsaaTargets,
    swapchain_format: SDL_GPUTextureFormat,
}

impl Renderer {
    pub fn new(device: &Device) -> Self {
        let mut vertex_shader = device.create_shader(&ShaderCreateInfo {
            code: include_bytes!("textured.vert.spv"),
            entrypoint: "main",
            format: SDL_GPUShaderFormat::SPIRV,
            stage: SDL_GPUShaderStage::VERTEX,
            num_samplers: 0,
            num_storage_textures: 0,
            num_storage_buffers: 0,
            num_uniform_buffers: 0,
        }).expect("Failed to create vertex shader");

        let mut fragment_shader = device.create_shader(&ShaderCreateInfo {
            code: include_bytes!("textured.frag.spv"),
            entrypoint: "main",
            format: SDL_GPUShaderFormat::SPIRV,
            stage: SDL_GPUShaderStage::FRAGMENT,
            num_samplers: 1,
            num_storage_textures: 0,
            num_storage_buffers: 0,
            num_uniform_buffers: 1,
        }).expect("Failed to create fragment shader");

        let swapchain_format = device.get_swapchain_texture_format();

        let pipeline = device.create_graphics_pipeline(&GraphicsPipelineCreateInfo {
            vertex_shader,
            fragment_shader,
            vertex_attributes: vec![
                SDL_GPUVertexAttribute {
                    location: 0,
                    buffer_slot: 0,
                    format: SDL_GPUVertexElementFormat::FLOAT2,
                    offset: 0,
                },
                SDL_GPUVertexAttribute {
                    location: 1,
                    buffer_slot: 0,
                    format: SDL_GPUVertexElementFormat::FLOAT2,
                    offset: (std::mem::size_of::<[f32; 2]>()) as u32,
                },
            ],
            vertex_buffer_descriptions: vec![
                SDL_GPUVertexBufferDescription {
                    slot: 0,
                    pitch: std::mem::size_of::<Vertex>() as u32,
                    input_rate: SDL_GPUVertexInputRate::VERTEX,
                    instance_step_rate: 0,
                },
            ],
            primitive_type: SDL_GPUPrimitiveType::TRIANGLELIST,
            rasterizer_state: Default::default(),
            multisample_state: SDL_GPUMultisampleState {
                sample_count: SAMPLE_COUNT,
                sample_mask: 0,
                enable_mask: false,
                enable_alpha_to_coverage: false,
                ..Default::default()
            },
            depth_stencil_state: Default::default(),
            color_target_descriptions: vec![SDL_GPUColorTargetDescription {
                format: swapchain_format,
                blend_state: Default::default(),
            }],
            depth_stencil_format: Default::default(),
            has_depth_stencil_target: false,
        }).expect("Failed to create graphics pipeline");

        vertex_shader.destroy(device);
        fragment_shader.destroy(device);

        // Vertex buffer
        let vertex_data_size = std::mem::size_of_val(&QUAD_VERTICES) as u32;
        let vertex_buffer = device.create_buffer(SDL_GPUBufferUsageFlags::VERTEX, vertex_data_size)
            .expect("Failed to create vertex buffer");
        device.upload_to_buffer(None, vertex_buffer, 0, bytemuck::cast_slice(&QUAD_VERTICES))
            .expect("Failed to upload vertex data");

        // Index buffer
        let index_data_size = std::mem::size_of_val(&QUAD_INDICES) as u32;
        let index_buffer = device.create_buffer(SDL_GPUBufferUsageFlags::INDEX, index_data_size)
            .expect("Failed to create index buffer");
        device.upload_to_buffer(None, index_buffer, 0, bytemuck::cast_slice(&QUAD_INDICES))
            .expect("Failed to upload index data");

        // Checkerboard texture
        let tex_size = 256u32;
        let cell_size = 32u32;
        let pixels = generate_checkerboard(tex_size, tex_size, cell_size);

        let checkerboard_texture = device.create_texture(&gpu::SDL_GPUTextureCreateInfo {
            r#type: SDL_GPUTextureType::_2D,
            format: SDL_GPUTextureFormat::R8G8B8A8_UNORM,
            usage: SDL_GPUTextureUsageFlags::SAMPLER,
            width: tex_size,
            height: tex_size,
            layer_count_or_depth: 1,
            num_levels: 1,
            sample_count: SDL_GPUSampleCount::_1,
            props: sdl3_gs::sys::properties::SDL_PropertiesID(0),
        }).expect("Failed to create checkerboard texture");

        let region = TextureRegion::full(checkerboard_texture, device);
        device.upload_to_texture(None, &region, &pixels)
            .expect("Failed to upload checkerboard data");

        // Sampler
        #[allow(deprecated)]
        let sampler = device.create_sampler(&gpu::SDL_GPUSamplerCreateInfo {
            min_filter: SDL_GPUFilter::LINEAR,
            mag_filter: SDL_GPUFilter::LINEAR,
            mipmap_mode: SDL_GPUSamplerMipmapMode::LINEAR,
            address_mode_u: SDL_GPUSamplerAddressMode::REPEAT,
            address_mode_v: SDL_GPUSamplerAddressMode::REPEAT,
            address_mode_w: SDL_GPUSamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            max_anisotropy: 1.0,
            compare_op: SDL_GPUCompareOp::NEVER,
            min_lod: 0.0,
            max_lod: 0.0,
            enable_anisotropy: false,
            enable_compare: false,
            padding1: 0,
            padding2: 0,
            props: sdl3_gs::sys::properties::SDL_PropertiesID(0),
        }).expect("Failed to create sampler");

        let targets = MsaaTargets::create(device, 1280, 720, swapchain_format);

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            checkerboard_texture,
            sampler,
            targets,
            swapchain_format,
        }
    }

    fn render_frame(&mut self, device: &Device) -> Result<(), &'static str> {
        let mut cmd = device.acquire_command_buffer()?;
        let Some(_swapchain) = cmd.acquire_swapchain_texture()? else {
            return Ok(());
        };

        let (sw, sh) = cmd.device().texture_res(Texture::SWAPCHAIN);

        if self.targets.width != sw || self.targets.height != sh {
            self.targets.destroy(device);
            self.targets = MsaaTargets::create(device, sw, sh, self.swapchain_format);
        }

        let mut target = ColorTargetInfo::new(self.targets.msaa);
        target.clear_color = SDL_FColor { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
        target.load_op = SDL_GPULoadOp::CLEAR;
        target.store_op = SDL_GPUStoreOp::RESOLVE;
        target.resolve_texture = Some(Texture::SWAPCHAIN);
        target.cycle = true;
        target.cycle_resolve_texture = true;

        let pass = cmd.begin_render_pass(&[target], None)?;
        pass.bind_graphics_pipeline(self.pipeline);
        pass.bind_vertex_buffers(0, &[GPUBufferBinding { buffer: self.vertex_buffer, offset: 0 }]);
        pass.bind_index_buffer(&GPUBufferBinding { buffer: self.index_buffer, offset: 0 }, SDL_GPUIndexElementSize::_16BIT);
        pass.bind_fragment_samplers(0, &[TextureSamplerBinding {
            texture: self.checkerboard_texture,
            sampler: self.sampler,
        }]);
        let tint_color: [f32; 4] = [1.0, 0.8, 0.5, 1.0];
        pass.push_fragment_uniform_data(0, bytemuck::cast_slice(&tint_color));
        pass.draw_indexed_primitives(6, 1, 0, 0, 0);
        drop(pass);

        // cmd.blit_texture(&BlitInfo::new(
        //     BlitRegion::full(self.targets.resolve, sw, sh),
        //     BlitRegion::full(Texture::SWAPCHAIN, sw, sh),
        // ));

        cmd.submit();
        Ok(())
    }
}

fn run_compute_fill(device: &Device) {
    let mut pipeline = device.create_compute_pipeline(&ComputePipelineCreateInfo {
        code: include_bytes!("fill_array.comp.spv"),
        entrypoint: "main",
        format: SDL_GPUShaderFormat::SPIRV,
        num_samplers: 0,
        num_readonly_storage_textures: 0,
        num_readonly_storage_buffers: 0,
        num_readwrite_storage_textures: 0,
        num_readwrite_storage_buffers: 1,
        num_uniform_buffers: 0,
        threadcount_x: 64,
        threadcount_y: 1,
        threadcount_z: 1,
    }).expect("Failed to create compute pipeline");

    let mut buffer = device.create_buffer(
        SDL_GPUBufferUsageFlags::COMPUTE_STORAGE_WRITE,
        256 * std::mem::size_of::<u32>() as u32,
    ).expect("Failed to create compute buffer");

    let mut cmd = device.acquire_command_buffer().expect("Failed to acquire command buffer");
    {
        let pass = cmd.begin_compute_pass(
            &[],
            &[StorageBufferReadWriteBinding { buffer, cycle: false }],
        ).expect("Failed to begin compute pass");
        pass.bind_compute_pipeline(pipeline);
        pass.dispatch(256 / 64, 1, 1);
    }
    cmd.submit().expect("Failed to submit compute command buffer");

    let data = device.download_from_buffer(buffer, 0, 0)
        .expect("Failed to download buffer");
    let values: &[u32] = bytemuck::cast_slice(&data);
    
    if false 
    {
        println!("Compute shader output ({} values):", values.len());
        for (i, v) in values.iter().enumerate() {
            if i > 0 && i % 16 == 0 {
                println!();
            }
            print!("{v:4}");
        }
        println!();
    }

    buffer.destroy(device);
    pipeline.destroy(device);
}

struct DemoApp {
    device: Device,
    renderer: Renderer,
}

impl App for DemoApp {
    fn init() -> Result<Self, String> {
        sdl3_gs::sdl_init(sdl3_gs::SDL_INIT_VIDEO);

        let window = sdl3_gs::window::Window::create(
            "hello GS", (1280, 720),
            sdl3_gs::SDL_WindowFlags::default()
                | sdl3_gs::SDL_WindowFlags::VULKAN
                | sdl3_gs::SDL_WindowFlags::RESIZABLE,
        )?;

        let mut device = Device::new(SDL_GPUShaderFormat::SPIRV, Some(window))
            .map_err(|e| e.to_string())?;

        let renderer = Renderer::new(&mut device);

        run_compute_fill(&device);

        Ok(DemoApp { device, renderer })
    }

    fn iterate(&mut self) -> bool {
        if let Err(e) = self.renderer.render_frame(&self.device) {
            eprintln!("Render error: {e}");
            return false;
        }
        true
    }

    fn event(&mut self, event: Event) -> bool {
        match event {
            Event::Quit { .. } => return false,
            Event::Window { kind: WindowEventKind::CloseRequested, .. } => return false,
            Event::KeyDown { scancode, repeat, .. } => {
                if scancode == SDL_Scancode::ESCAPE {
                    return false;
                }
                if !repeat {
                    println!("Key pressed: {:?}", scancode.0);
                }
            }
            Event::MouseButtonDown { button, x, y, .. } => {
                println!("Mouse button {} down at ({}, {})", button, x, y);
            }
            _ => {}
        }
        true
    }

    fn quit(&mut self) {}
}

sdl3_gs::sdl3_main!(DemoApp);
