#![allow(unused)]
use sdl3_gs::device::*;
use sdl3_gs::event::{Event, WindowEventKind, SDL_Scancode};
use sdl3_gs::sys::gpu;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
}

const TRIANGLE_VERTICES: [Vertex; 3] = [
    Vertex { pos: [ 0.0, -0.5], color: [1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5,  0.5], color: [0.0, 1.0, 0.0] },
    Vertex { pos: [-0.5,  0.5], color: [0.0, 0.0, 1.0] },
];

const SAMPLE_COUNT: SDL_GPUSampleCount = SDL_GPUSampleCount::_4;

struct MsaaTargets {
    msaa: Texture,
    resolve: Texture,
    width: u32,
    height: u32,
}

impl MsaaTargets {
    fn create(device: &Device, width: u32, height: u32, format: SDL_GPUTextureFormat) -> Self {
        let msaa = device.create_texture(&gpu::SDL_GPUTextureCreateInfo {
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

        let resolve = device.create_texture(&gpu::SDL_GPUTextureCreateInfo {
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

    fn destroy(&self, device: &Device) {
        device.destroy_texture(self.msaa);
        device.destroy_texture(self.resolve);
    }
}

struct Renderer {
    pipeline: GraphicsPipeline,
    vertex_buffer: GPUBuffer,
    targets: MsaaTargets,
    swapchain_format: SDL_GPUTextureFormat,
}

impl Renderer {
    pub fn new(device: &mut Device) -> Self {
        let vertex_shader = device.create_shader(&ShaderCreateInfo {
            code: include_bytes!("triangle.vert.spv"),
            entrypoint: "main",
            format: SDL_GPUShaderFormat::SPIRV,
            stage: SDL_GPUShaderStage::VERTEX,
            num_samplers: 0,
            num_storage_textures: 0,
            num_storage_buffers: 0,
            num_uniform_buffers: 0,
        }).expect("Failed to create vertex shader");

        let fragment_shader = device.create_shader(&ShaderCreateInfo {
            code: include_bytes!("triangle.frag.spv"),
            entrypoint: "main",
            format: SDL_GPUShaderFormat::SPIRV,
            stage: SDL_GPUShaderStage::FRAGMENT,
            num_samplers: 0,
            num_storage_textures: 0,
            num_storage_buffers: 0,
            num_uniform_buffers: 0,
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
                    format: SDL_GPUVertexElementFormat::FLOAT3,
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
            multisample_state: gpu::SDL_GPUMultisampleState {
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

        device.destroy_shader(vertex_shader);
        device.destroy_shader(fragment_shader);

        // Create vertex buffer
        let vertex_data_size = std::mem::size_of_val(&TRIANGLE_VERTICES) as u32;
        let vertex_buffer = device.create_buffer(SDL_GPUBufferUsageFlags::VERTEX, vertex_data_size)
            .expect("Failed to create vertex buffer");

        // Upload vertex data
        let vertex_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                TRIANGLE_VERTICES.as_ptr() as *const u8,
                vertex_data_size as usize,
            )
        };
        device.upload_to_buffer(None, vertex_buffer, 0, vertex_bytes)
            .expect("Failed to upload vertex data");

        // Create initial MSAA targets at a default size (will be recreated on first frame)
        let targets = MsaaTargets::create(device, 1280, 720, swapchain_format);

        Self {
            pipeline,
            vertex_buffer,
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

        // Recreate MSAA targets if swapchain size changed
        if self.targets.width != sw || self.targets.height != sh {
            self.targets.destroy(device);
            self.targets = MsaaTargets::create(device, sw, sh, self.swapchain_format);
        }

        // Render to MSAA target, resolve into resolve texture
        let mut target = ColorTargetInfo::new(self.targets.msaa);
        target.clear_color = SDL_FColor { r: 0.25, g: 0.5, b: 0.25, a: 1.0 };
        target.load_op = SDL_GPULoadOp::CLEAR;
        target.store_op = SDL_GPUStoreOp::RESOLVE;
        target.resolve_texture = Some(self.targets.resolve);
        target.cycle = true;
        target.cycle_resolve_texture = true;

        let pass = cmd.begin_render_pass(&[target], None)?;
        pass.bind_graphics_pipeline(self.pipeline);
        pass.bind_vertex_buffers(0, &[GPUBufferBinding { buffer: self.vertex_buffer, offset: 0 }]);
        pass.draw_primitives(3, 1, 0, 0);
        drop(pass);

        // Blit resolved texture to swapchain
        cmd.blit_texture(&BlitInfo::new(
            BlitRegion::full(self.targets.resolve, sw, sh),
            BlitRegion::full(Texture::SWAPCHAIN, sw, sh),
        ));

        cmd.submit();
        Ok(())
    }
}

fn main() {
    sdl3_gs::sdl_init(sdl3_gs::SDL_INIT_VIDEO);

    let window = sdl3_gs::window::Window::create(
        "hello GS", (1280, 720),
        sdl3_gs::SDL_WindowFlags::default() | sdl3_gs::SDL_WindowFlags::VULKAN | sdl3_gs::SDL_WindowFlags::RESIZABLE).unwrap();

    let mut device = sdl3_gs::device::Device::new(sdl3_gs::device::SDL_GPUShaderFormat::SPIRV, Some(window)).unwrap();

    let mut renderer = Renderer::new(&mut device);

    let mut running = true;
    while running {
        for event in sdl3_gs::event::poll_events() {
            match event {
                Event::Quit { .. } => running = false,

                Event::Window { kind: WindowEventKind::CloseRequested, .. } => running = false,

                Event::KeyDown { scancode, repeat, .. } => {
                    if scancode == SDL_Scancode::ESCAPE {
                        running = false;
                    }
                    if !repeat {
                        println!("Key pressed: {:?}", scancode.0);
                    }
                }

                Event::MouseButtonDown { button, x, y, .. } => {
                    println!("Mouse button {} down at ({}, {})", button, x, y);
                }

                Event::MouseMotion { x, y, .. } => {
                    // Uncomment to see mouse motion:
                    // println!("Mouse at ({}, {})", x, y);
                }

                _ => {}
            }
        }


        renderer.render_frame(&mut device);

    }
}
