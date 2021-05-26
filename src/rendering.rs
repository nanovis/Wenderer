use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::data::Uniforms;
use crate::geometries::{Mesh3, Rectangle};
use crate::shading::Tex;
use crate::utils::create_cube_fbo;
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
    fn vertex_desc(&self) -> VertexBufferLayout;
    fn get_vertex_raw(&self) -> &[u8];
    fn get_index_raw(&self) -> &[u8];
    fn get_index_format(&self) -> IndexFormat;
    fn get_num_indices(&self) -> usize;
}

pub trait RenderPass {
    fn resize(&mut self, device: &Device, sc_desc: &SwapChainDescriptor);
    fn render(
        &self,
        render_into_view: &TextureView,
        depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
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
        return proj * view;
    }
}

pub struct D3Pass {
    depth_texture: Tex,
    uniform_bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniforms: Uniforms,
    uniform_buffer: Buffer,
    num_depth_indices: u32,
    render_pipeline: RenderPipeline,
    depth_clear_op: LoadOp<f32>,
    pub clear_color: (f64, f64, f64, f64),
    cube: Mesh3,
}

impl D3Pass {
    pub fn new(
        device: &Device,
        queue: &Queue,
        sc_desc: &SwapChainDescriptor,
        target_format: &TextureFormat,
        render_front_face: bool,
        camera: &Camera,
    ) -> Self {
        // configuring back and front face rendering
        let face_render_config = if render_front_face {
            (Face::Back, CompareFunction::Less, LoadOp::Clear(1.0))
        } else {
            (Face::Front, CompareFunction::Greater, LoadOp::Clear(0.0))
        };
        let depth_clear_op = face_render_config.2;
        // create geometry
        let cube = create_cube_fbo();
        // create depth texture
        let depth_texture = Tex::create_depth_texture(&device, &sc_desc, "depth_texture");
        // create uniforms
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(camera);
        let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: uniforms.as_std140().as_bytes(),
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            label: Some("uniform_bind_group"),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: cube.get_vertex_raw(),
            usage: BufferUsage::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: cube.get_index_raw(),
            usage: BufferUsage::INDEX,
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let vs_module = device.create_shader_module(&include_spirv!("shaders/shader.vert.spv"));
        let fs_module = device.create_shader_module(&include_spirv!("shaders/shader.frag.spv"));

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[cube.vertex_desc()],
            },
            fragment: Some(FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: target_format.clone(),
                    blend: Some(BlendState::REPLACE), //specify that the blending should just replace old pixel data with new data
                    write_mask: ColorWrite::ALL, //tell wgpu to write to all colors: red, blue, green, and alpha
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw, // facing forward if the vertices are arranged in a counter clockwise direction
                cull_mode: Some(face_render_config.0),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: Tex::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: face_render_config.1, // tells us when to discard a new pixel
                stencil: StencilState::default(),
                bias: DepthBiasState {
                    constant: 2, // Corresponds to bilinear filtering
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: 1,                         // not using multisampling
                mask: !0,                         // use all samples
                alpha_to_coverage_enabled: false, // related to anti-aliasing, not using for now
            }, // the config of this struct is the same as MultisampleState::default()
        });
        Self {
            depth_texture,
            vertex_buffer,
            index_buffer,
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            depth_clear_op,
            clear_color: (0.0, 0.0, 0.0, 1.0),
            num_depth_indices: cube.get_num_indices() as u32,
            render_pipeline,
            cube,
        }
    }

    pub fn update_view_proj_uniform(&mut self, camera: &Camera, queue: &Queue) {
        self.uniforms.update_view_proj(camera);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            self.uniforms.as_std140().as_bytes(),
        );
    }
}

impl RenderPass for D3Pass {
    fn resize(&mut self, device: &Device, sc_desc: &SwapChainDescriptor) {
        self.depth_texture = Tex::create_depth_texture(device, sc_desc, "depth texture");
    }

    fn render(
        &self,
        render_into_view: &TextureView,
        external_depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view: render_into_view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target: None,
                ops: Operations {
                    // The load field tells wgpu how to handle colors stored from the previous frame
                    load: LoadOp::Clear(Color {
                        r: self.clear_color.0,
                        g: self.clear_color.1,
                        b: self.clear_color.2,
                        a: self.clear_color.3,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: external_depth_view.unwrap_or(&self.depth_texture.view),
                depth_ops: Some(Operations {
                    load: self.depth_clear_op.clone(),
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
        render_pass.set_index_buffer(self.index_buffer.slice(..), self.cube.get_index_format());
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}

pub struct VanillaPass {
    texture_bind_group_layout: BindGroupLayout,
    texture_bind_group: BindGroup,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_depth_indices: u32,
    render_pipeline: RenderPipeline,
    canvas: Rectangle,
}

impl VanillaPass {
    pub fn new(image_texture: &Tex, device: &Device, target_format: &TextureFormat) -> Self {
        let canvas = Rectangle::new_unit_rectangle();
        // A BindGroup describes a set of resources and how they can be accessed by a shader.
        // We create a BindGroup using a BindGroupLayout.
        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Texture binding group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler {
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
        let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&image_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&image_texture.sampler),
                },
            ],
        });
        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: canvas.get_vertex_raw(),
            usage: BufferUsage::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: canvas.get_index_raw(),
            usage: BufferUsage::INDEX,
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let vs_module = device.create_shader_module(&include_spirv!("shaders/vanilla.vert.spv"));
        let fs_module = device.create_shader_module(&include_spirv!("shaders/vanilla.frag.spv"));

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[canvas.vertex_desc()],
            },
            fragment: Some(FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format: target_format.clone(),
                    blend: Some(BlendState::REPLACE), //specify that the blending should just replace old pixel data with new data
                    write_mask: ColorWrite::ALL, //tell wgpu to write to all colors: red, blue, green, and alpha
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw, // facing forward if the vertices are arranged in a counter clockwise direction
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
        });
        Self {
            texture_bind_group_layout,
            texture_bind_group,
            vertex_buffer,
            index_buffer,
            num_depth_indices: canvas.get_num_indices() as u32,
            canvas,
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
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view: render_into_view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target: None,
                ops: Operations {
                    // The load field tells wgpu how to handle colors stored from the previous frame
                    load: LoadOp::Clear(Color {
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
        render_pass.set_index_buffer(self.index_buffer.slice(..), self.canvas.get_index_format());
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}
