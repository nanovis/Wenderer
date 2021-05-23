use crate::data::Vertex;
use crate::rendering::Geometry;
use std::mem::size_of;
use wgpu::VertexBufferLayout;

const DEFAULT_VERTEX_LAYOUT: VertexBufferLayout = wgpu::VertexBufferLayout {
    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
    step_mode: wgpu::InputStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0, // corresponds to layout(location = 0) in shader
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x2,
        },
    ],
};

pub struct Pentagon;

impl Pentagon {
    const VERTICES: &'static [Vertex] = &[
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

    const INDICES: &'static [u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
}

impl Geometry for Pentagon {
    fn vertex_desc(&self) -> VertexBufferLayout {
        DEFAULT_VERTEX_LAYOUT
    }

    fn get_vertex_raw(&self) -> &[u8] {
        bytemuck::cast_slice(Self::VERTICES)
    }

    fn get_index_raw(&self) -> &[u8] {
        bytemuck::cast_slice(Self::INDICES)
    }

    fn get_num_indices(&self) -> usize {
        Self::INDICES.len()
    }
}

pub struct Rectangle;

impl Rectangle {
    const VERTICES: &'static [Vertex] = &[
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

    const INDICES: &'static [u16] = &[0, 1, 2, 0, 2, 3];
}

impl Geometry for Rectangle {
    fn vertex_desc(&self) -> VertexBufferLayout {
        DEFAULT_VERTEX_LAYOUT
    }

    fn get_vertex_raw(&self) -> &[u8] {
        bytemuck::cast_slice(Self::VERTICES)
    }

    fn get_index_raw(&self) -> &[u8] {
        bytemuck::cast_slice(Self::INDICES)
    }

    fn get_num_indices(&self) -> usize {
        Self::INDICES.len()
    }
}
