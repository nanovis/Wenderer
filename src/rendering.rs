use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::data::{CanvasShaderUniforms, Uniforms};
use crate::geometries::{Mesh3, Rectangle};
use crate::shading::Tex;
use crate::utils::{create_cube_fbo, load_example_transfer_function};
use crevice::std140::AsStd140;
use std::num::NonZeroU32;

// The coordinate system in Wgpu is based on DirectX, and Metal's coordinate systems.
// That means that in normalized device coordinates the x axis and y axis are in the range of -1.0 to +1.0, and the z axis is 0.0 to +1.0.
// The cgmath crate (as well as most game math crates) are built for OpenGL's coordinate system.
// This matrix will scale and translate our scene from OpenGL's coordinate system to WGPU's.
// We'll define it as follows.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
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
    fn resize(&mut self, device: &Device, width: u32, height: u32);
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
    pub fn build_view_projection_matrix(&self, model_transformation: Matrix4<f32>) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.eye, self.center, self.up);
        let proj = perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);
        return proj * view * model_transformation;
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
    multisample_buffer: Option<Tex>,
    pub clear_color: (f64, f64, f64, f64),
    cube: Mesh3,
    sample_count: u32,
}

impl D3Pass {
    pub fn new(
        device: &Device,
        render_width: u32,
        render_height: u32,
        target_format: &TextureFormat,
        render_front_face: bool,
        camera: &Camera,
        sample_cnt: NonZeroU32,
        cube_transformation: Matrix4<f32>,
    ) -> Self {
        let sample_count = sample_cnt.get();
        let enable_multisample = sample_count > 1;
        let multisample_buffer = if enable_multisample {
            Some(Tex::create_render_buffer(
                (render_width, render_height),
                device,
                Some("Multisample Buffer"),
                sample_cnt.clone(),
                target_format,
            ))
        } else {
            None
        };
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
        let depth_texture = Tex::create_depth_texture(
            &device,
            render_width,
            render_height,
            sample_cnt,
            "depth_texture",
        );
        // create uniforms
        let mut uniforms = Uniforms::new();
        uniforms.update_model_view_proj(camera, cube_transformation);
        let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: uniforms.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
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
            usage: BufferUsages::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: cube.get_index_raw(),
            usage: BufferUsages::INDEX,
        });

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("3D Pass shaders"),
            source: ShaderSource::Wgsl(include_str!("./shaders/shader3d.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("3DPass Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("3DPass Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex_shader",
                buffers: &[cube.vertex_desc()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment_shader",
                targets: &[Some(ColorTargetState {
                    format: target_format.clone(),
                    blend: Some(BlendState::REPLACE), //specify that the blending should just replace old pixel data with new data
                    write_mask: ColorWrites::ALL, //tell wgpu to write to all colors: red, blue, green, and alpha
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw, // facing forward if the vertices are arranged in a counter clockwise direction
                cull_mode: Some(face_render_config.0),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
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
                count: sample_count,
                ..Default::default()
            }, // the config of this struct is the same as MultisampleState::default()
            multiview: None,
        });
        Self {
            depth_texture,
            vertex_buffer,
            index_buffer,
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            depth_clear_op,
            multisample_buffer,
            clear_color: (0.0, 0.0, 0.0, 1.0),
            num_depth_indices: cube.get_num_indices() as u32,
            render_pipeline,
            cube,
            sample_count,
        }
    }

    pub fn update_model_view_proj_uniform(
        &mut self,
        model_transformation: Matrix4<f32>,
        camera: &Camera,
        queue: &Queue,
    ) {
        self.uniforms
            .update_model_view_proj(camera, model_transformation);
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            self.uniforms.as_std140().as_bytes(),
        );
    }
}

impl RenderPass for D3Pass {
    fn resize(&mut self, device: &Device, render_width: u32, render_height: u32) {
        let sample_cnt = NonZeroU32::new(self.sample_count).unwrap();
        self.depth_texture = Tex::create_depth_texture(
            device,
            render_width,
            render_height,
            sample_cnt.clone(),
            "depth texture",
        );
        self.multisample_buffer = match self.multisample_buffer {
            None => None,
            Some(ref old_buffer) => Some(Tex::create_render_buffer(
                (render_width, render_height),
                device,
                Some("Multisample Buffer"),
                sample_cnt,
                &old_buffer.format,
            )),
        }
    }

    fn render(
        &self,
        render_into_view: &TextureView,
        external_depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let (view, resolve_target) = match self.multisample_buffer {
            None => (render_into_view, None),
            Some(ref multisample_buffer) => (&multisample_buffer.view, Some(render_into_view)),
        };
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[Some(RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target,
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
            })],
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

pub struct CanvasPass {
    face_texture_bind_group_layout: BindGroupLayout,
    face_texture_bind_group: BindGroup,
    volume_bind_group: BindGroup,
    tf_bind_group: BindGroup,
    uniforms: CanvasShaderUniforms,
    uniform_bind_group: BindGroup,
    uniform_buffer: Buffer,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_depth_indices: u32,
    render_pipeline: RenderPipeline,
    canvas: Rectangle,
    sample_count: u32,
    multisample_buffer: Option<Tex>,
}

impl CanvasPass {
    pub fn new(
        front_face_render_buffer: &Tex,
        back_face_render_buffer: &Tex,
        volume_texture: &Tex,
        device: &Device,
        queue: &Queue,
        resolution: (u32, u32),
        tex_format: &TextureFormat,
        sample_cnt: NonZeroU32,
    ) -> Self {
        let sample_count = sample_cnt.get();
        let multisample_buffer = if sample_count > 1 {
            Some(Tex::create_render_buffer(
                resolution,
                device,
                Some("Render Pass multisample buffer"),
                sample_cnt,
                tex_format,
            ))
        } else {
            None
        };
        let canvas = Rectangle::new_standard_rectangle();
        // A BindGroup describes a set of resources and how they can be accessed by a shader.
        // We create a BindGroup using a BindGroupLayout.
        let face_texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Texture binding group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        // That's because a BindGroup is a more specific declaration of the BindGroupLayout.
        // The reason why they're separate is it allows us to swap out BindGroups on the fly,
        // so long as they all share the same BindGroupLayout
        let face_texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Backface and front face bind group"),
            layout: &face_texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&front_face_render_buffer.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&front_face_render_buffer.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&back_face_render_buffer.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&back_face_render_buffer.sampler),
                },
            ],
        });
        let volume_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("volume bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D3,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let volume_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("volume bind group"),
            layout: &volume_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&volume_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&volume_texture.sampler),
                },
            ],
        });
        let transfer_function_values = load_example_transfer_function();
        let transfer_function_texture = Tex::create_1d_texture_rgba8(
            &transfer_function_values,
            device,
            queue,
            "Transfer function",
        );
        let tf_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tf bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D1,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let tf_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("volume bind group"),
            layout: &tf_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&transfer_function_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&transfer_function_texture.sampler),
                },
            ],
        });
        // create uniform bindings
        let uniforms = CanvasShaderUniforms::default();
        let uniform_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: uniforms.as_std140().as_bytes(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        // create vertex buffer
        let vertex_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: canvas.get_vertex_raw(),
            usage: BufferUsages::VERTEX,
        });
        // create index buffer
        let index_buffer = device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: canvas.get_index_raw(),
            usage: BufferUsages::INDEX,
        });

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Canvas Pass Shaders"),
            source: ShaderSource::Wgsl(include_str!("./shaders/canvas_shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Canvas Pass Render Pipeline Layout"),
            bind_group_layouts: &[
                &face_texture_bind_group_layout,
                &volume_bind_group_layout,
                &tf_bind_group_layout,
                &uniform_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Canvas Pass Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex_shader",
                buffers: &[canvas.vertex_desc()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment_shader",
                targets: &[Some(ColorTargetState {
                    format: tex_format.clone(),
                    blend: Some(BlendState::REPLACE), //specify that the blending should just replace old pixel data with new data
                    write_mask: ColorWrites::ALL, //tell wgpu to write to all colors: red, blue, green, and alpha
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw, // facing forward if the vertices are arranged in a counter clockwise direction
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
        });
        Self {
            face_texture_bind_group_layout,
            face_texture_bind_group,
            volume_bind_group,
            tf_bind_group,
            uniforms,
            uniform_bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            num_depth_indices: canvas.get_num_indices() as u32,
            canvas,
            render_pipeline,
            sample_count,
            multisample_buffer,
        }
    }

    pub fn change_bound_face_textures(
        &mut self,
        device: &Device,
        front_face_texture: &Tex,
        back_face_texture: &Tex,
    ) {
        self.face_texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Backface and front face bind group"),
            layout: &self.face_texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&front_face_texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&front_face_texture.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&back_face_texture.view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&back_face_texture.sampler),
                },
            ],
        });
    }

    pub fn set_uniforms(&mut self, uniforms: &CanvasShaderUniforms, queue: &Queue) {
        self.uniforms = uniforms.clone();
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            self.uniforms.as_std140().as_bytes(),
        );
    }
}

impl RenderPass for CanvasPass {
    fn resize(&mut self, device: &Device, width: u32, height: u32) {
        self.multisample_buffer = match self.multisample_buffer {
            None => None,
            Some(ref old_buffer) => Some(Tex::create_render_buffer(
                (width, height),
                device,
                Some("Render Pass multisample buffer"),
                NonZeroU32::new(self.sample_count).unwrap(),
                &old_buffer.format,
            )),
        }
    }

    fn render(
        &self,
        render_into_view: &TextureView,
        _depth_view: Option<&TextureView>,
        encoder: &mut CommandEncoder,
    ) {
        let (view, resolve_target) = match self.multisample_buffer {
            None => (render_into_view, None),
            Some(ref multisample_buffer) => (&multisample_buffer.view, Some(render_into_view)),
        };
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            // color_attachments describe where we are going to draw our color to
            color_attachments: &[Some(RenderPassColorAttachment {
                //view informs wgpu what texture to save the colors to
                view,
                // The resolve_target is the texture that will receive the resolved output.
                // This will be the same as `view` unless multisampling is enabled
                resolve_target,
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
            })],
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
        render_pass.set_bind_group(0, &self.face_texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.volume_bind_group, &[]);
        render_pass.set_bind_group(2, &self.tf_bind_group, &[]);
        render_pass.set_bind_group(3, &self.uniform_bind_group, &[]);
        render_pass.draw_indexed(0..self.num_depth_indices, 0, 0..1);
    }
}
