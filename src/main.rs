use futures::executor::block_on;
use wenderer::data::{Uniforms, Vertex};
use wenderer::rendering::Camera;
use wenderer::shading::Texture;
use wenderer::utils::CameraController;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    },
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    },
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397057],
    },
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732911],
    },
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    },
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

const DEPTH_VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
];

const DEPTH_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

struct DepthPass {
    texture: Texture,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_depth_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl DepthPass {
    fn new(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) -> Self {
        let texture = Texture::create_depth_texture(device, sc_desc, "depth_texture");
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Pass Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    visibility: wgpu::ShaderStage::FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    ty: wgpu::BindingType::Sampler {
                        comparison: true,
                        filtering: true,
                    },
                    visibility: wgpu::ShaderStage::FRAGMENT,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            label: Some("depth_pass.bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Pass VB"),
            contents: bytemuck::cast_slice(DEPTH_VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Pass IB"),
            contents: bytemuck::cast_slice(DEPTH_INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Depth Pass Pipeline Layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/challenge.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/challenge.frag.spv"));
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Depth Pass Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });
        Self {
            texture,
            layout,
            bind_group,
            vertex_buffer,
            index_buffer,
            num_depth_indices: DEPTH_INDICES.len() as u32,
            render_pipeline,
        }
    }

    fn resize(&mut self, device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) {
        self.texture = Texture::create_depth_texture(device, sc_desc, "depth_texture");
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            label: Some("depth_pass.bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture.sampler),
                },
            ],
        });
    }

    fn render(&self, frame: &wgpu::SwapChainTexture, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Depth Visual Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: (f64, f64, f64, f64),
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: Texture,
    depth_texture: Texture,
    camera: Camera,
    camera_controller: CameraController,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_pass: DepthPass,
}

impl State {
    // need async because we need to await some struct creation here
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        // need adapter to create the device and queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(), //The device you have limits the features you can use
                    limits: wgpu::Limits::default(), //The limits field describes the limit of certain types of resource we can create
                    label: None,
                },
                None,
            )
            .await
            .unwrap();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT, //RENDER_ATTACHMENT specifies that the textures will be used to write to the screen
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        // create texture
        let diffuse_bytes = include_bytes!("../data/happy-tree.png");
        let diffuse_texture =
            Texture::from_bytes(&device, &queue, diffuse_bytes, "happy_tree.png").unwrap();
        // create depth texture
        let depth_texture = Texture::create_depth_texture(&device, &sc_desc, "depth_texture");
        // A BindGroup describes a set of resources and how they can be accessed by a shader.
        // We create a BindGroup using a BindGroupLayout.
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture binding group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false, // mostly for depth texture
                            filtering: true,
                        },
                        count: None,
                    },
                ],
            });
        // That's because a BindGroup is a more specific declaration of the BindGroupLayout.
        // The reason why they're separate is it allows us to swap out BindGroups on the fly,
        // so long as they all share the same BindGroupLayout
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
        });
        // rendering configurations
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            center: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: (sc_desc.width as f32) / (sc_desc.height as f32),
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        // create uniforms
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        // HIGHLIGHT: question: can we use a same bind group with texture
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            label: Some("uniform_bind_group"),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });

        // create render pipeline with shaders
        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/shader.frag.spv"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE), //specify that the blending should just replace old pixel data with new data
                    write_mask: wgpu::ColorWrite::ALL, //tell wgpu to write to all colors: red, blue, green, and alpha
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // facing forward if the vertices are arranged in a counter clockwise direction
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // tells us when to discard a new pixel
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // Corresponds to bilinear filtering
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: 1,                         // not using multisampling
                mask: !0,                         // use all samples
                alpha_to_coverage_enabled: false, // related to anti-aliasing, not using for now
            }, // the config of this struct is the same as wgpu::MultisampleState::default()
        });
        let depth_pass = DepthPass::new(&device, &sc_desc);
        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            clear_color: (0.1, 0.2, 0.3, 1.0),
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            diffuse_bind_group,
            diffuse_texture,
            depth_texture,
            camera,
            camera_controller: CameraController::new(0.2),
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            depth_pass,
        }
    }
    // If we want to support resizing in our application, we're going to need to recreate the swap_chain everytime the window's size changes.
    // That's the reason we stored the physical size and the sc_desc used to create the swap chain.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.depth_texture =
            Texture::create_depth_texture(&self.device, &self.sc_desc, "depth texture");
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.depth_pass.resize(&self.device, &self.sc_desc);
        self.camera.aspect = self.sc_desc.width as f32 / self.sc_desc.height as f32;
    }
    // input() returns a bool to indicate whether an event has been fully processed.
    // If the method returns true, the main loop won't process the event any further.
    fn input(&mut self, event: &WindowEvent) -> bool {
        if self.camera_controller.process_events(event) {
            return true;
        }
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                if position.x > (self.size.width / 2) as f64 {
                    self.clear_color = (0.3, 0.2, 0.1, 1.0);
                } else {
                    self.clear_color = (0.1, 0.2, 0.3, 1.0);
                }
                return true;
            }
            _ => {}
        }
        return false;
    }
    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }
    // We also need to create a CommandEncoder to create the actual commands to send to the gpu.
    // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the gpu.
    // The encoder builds a command buffer that we can then send to the gpu.
    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[wgpu::RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view: &frame.view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target: None,
                ops: wgpu::Operations {
                    // The load field tells wgpu how to handle colors stored from the previous frame
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.clear_color.0,
                        g: self.clear_color.1,
                        b: self.clear_color.2,
                        a: self.clear_color.3,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_pass.texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });
        render_pass.set_pipeline(&self.render_pipeline);
        // set_vertex_buffer takes two parameters.
        // The first is what buffer slot to use for this vertex buffer.
        // You can have multiple vertex buffers set at a time
        // The second parameter is the slice of the buffer to use.
        // You can store as many objects in a buffer as your hardware allows, so slice allows us to specify which portion of the buffer to use.
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        drop(render_pass); // why? see learn-wgpu
        self.depth_pass.render(&frame, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            // see the explanation of State::input()
            if !state.input(event) {
                match event {
                    WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Recreate the swap_chain if lost
                Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("Some unhandled error {:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {}
    })
}
