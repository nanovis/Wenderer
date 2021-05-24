use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;
use wgpu::{CommandEncoder, Device, Queue, SwapChainDescriptor, TextureFormat, TextureView};

use crate::data::Uniforms;
use crate::geometries::{Pentagon, Rectangle};
use crate::shading::Texture;
use crevice::std140::{AsStd140, Std140};

// The coordinate system in Wgpu is based on DirectX, and Metal's coordinate systems.
// That means that in normalized device coordinates the x axis and y axis are in the range of -1.0 to +1.0, and the z axis is 0.0 to +1.0.
// The cgmath crate (as well as most game math crates) are built for OpenGL's coordinate system.
// This matrix will scale and translate our scene from OpenGL's coordinate sytem to WGPU's.
// We'll define it as follows.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub trait Geometry {
    fn vertex_desc(&self) -> wgpu::VertexBufferLayout;
    fn get_vertex_raw(&self) -> &[u8];
    fn get_index_raw(&self) -> &[u8];
    fn get_num_indices(&self) -> usize;
}

pub trait RenderPass {
    fn resize(&mut self, device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor);
    fn render(
        &self,
        render_into_view: &TextureView,
        depth_view: Option<&TextureView>,
        encoder: &mut wgpu::CommandEncoder,
    );
}

pub struct Camera {
    pub eye: Point3<f32>,
    pub center: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.eye, self.center, self.up);
        let proj = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

pub struct DepthPass {
    pub depth_texture: Texture,
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_depth_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl DepthPass {
    const CANVAS: Rectangle = Rectangle;

    pub fn new(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor) -> Self {
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
            contents: Self::CANVAS.get_vertex_raw(),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Pass IB"),
            contents: Self::CANVAS.get_index_raw(),
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
                buffers: &[Self::CANVAS.vertex_desc()],
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
            depth_texture: texture,
            layout,
            bind_group,
            vertex_buffer,
            index_buffer,
            num_depth_indices: Self::CANVAS.get_num_indices() as u32,
            render_pipeline,
        }
    }
}

impl RenderPass for DepthPass {
    fn resize(&mut self, device: &Device, sc_desc: &SwapChainDescriptor) {
        self.depth_texture = Texture::create_depth_texture(device, sc_desc, "depth_texture");
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            label: Some("depth_pass.bind_group"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.depth_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.depth_texture.sampler),
                },
            ],
        });
    }

    fn render(
        &self,
        render_into_view: &TextureView,
        _depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Depth Visual Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: render_into_view,
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

pub struct ColorPass {
    pub image_texture: Texture,
    depth_texture: Texture,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    num_depth_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
    pub clear_color: (f64, f64, f64, f64),
}

impl ColorPass {
    const GEOMETRY: Pentagon = Pentagon;

    pub fn new(
        device: &wgpu::Device,
        queue: &Queue,
        sc_desc: &wgpu::SwapChainDescriptor,
        target_format: &TextureFormat,
        camera: &Camera,
    ) -> Self {
        // create texture
        let diffuse_bytes = include_bytes!("../data/happy-tree.png");
        let image_texture =
            Texture::from_bytes(&device, queue, diffuse_bytes, "happy_tree.png").unwrap();
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
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&image_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&image_texture.sampler),
                },
            ],
        });
        // create uniforms
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(camera);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: uniforms.as_std140().as_bytes(),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
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
            contents: Self::GEOMETRY.get_vertex_raw(),
            usage: wgpu::BufferUsage::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: Self::GEOMETRY.get_index_raw(),
            usage: wgpu::BufferUsage::INDEX,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/shader.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/shader.frag.spv"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Self::GEOMETRY.vertex_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: target_format.clone(),
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
        Self {
            image_texture,
            depth_texture,
            texture_bind_group_layout,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            clear_color: (0.1, 0.2, 0.3, 1.0),
            num_depth_indices: Self::GEOMETRY.get_num_indices() as u32,
            render_pipeline,
        }
    }

    pub fn update_view_proj_uniform(&mut self, camera: &Camera, queue: &wgpu::Queue) {
        self.uniforms.update_view_proj(camera);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            self.uniforms.as_std140().as_bytes(),
        );
    }
}

impl RenderPass for ColorPass {
    fn resize(&mut self, device: &Device, sc_desc: &SwapChainDescriptor) {
        self.depth_texture = Texture::create_depth_texture(device, sc_desc, "depth texture");
    }

    fn render(
        &self,
        render_into_view: &TextureView,
        external_depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[wgpu::RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view: render_into_view,
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
                view: external_depth_view.unwrap_or(&self.depth_texture.view),
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
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}

pub struct VanillaPass {
    texture_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_depth_indices: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl VanillaPass {
    const GEOMETRY: Rectangle = Rectangle;

    pub fn new(
        image_texture: &Texture,
        device: &wgpu::Device,
        sc_desc: &wgpu::SwapChainDescriptor,
        target_format: &TextureFormat,
    ) -> Self {
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
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&image_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&image_texture.sampler),
                },
            ],
        });
        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: Self::GEOMETRY.get_vertex_raw(),
            usage: wgpu::BufferUsage::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: Self::GEOMETRY.get_index_raw(),
            usage: wgpu::BufferUsage::INDEX,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        let vs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/vanilla.vert.spv"));
        let fs_module =
            device.create_shader_module(&wgpu::include_spirv!("shaders/vanilla.frag.spv"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[Self::GEOMETRY.vertex_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: target_format.clone(),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });
        Self {
            texture_bind_group_layout,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            num_depth_indices: Self::GEOMETRY.get_num_indices() as u32,
            render_pipeline,
        }
    }
}

impl RenderPass for VanillaPass {
    fn resize(&mut self, device: &Device, sc_desc: &SwapChainDescriptor) {}

    fn render(
        &self,
        render_into_view: &TextureView,
        _depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[wgpu::RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view: render_into_view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target: None,
                ops: wgpu::Operations {
                    // The load field tells wgpu how to handle colors stored from the previous frame
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        // set_vertex_buffer takes two parameters.
        // The first is what buffer slot to use for this vertex buffer.
        // You can have multiple vertex buffers set at a time
        // The second parameter is the slice of the buffer to use.
        // You can store as many objects in a buffer as your hardware allows, so slice allows us to specify which portion of the buffer to use.
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}
