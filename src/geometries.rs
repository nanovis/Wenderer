use crate::data::{Vertex2, Vertex3};
use crate::rendering::{Geometry, OPENGL_TO_WGPU_MATRIX};
use cgmath::{Matrix4, Vector2, Vector3, Vector4};
use rayon::prelude::*;
use std::mem::size_of;
use wgpu::{BufferAddress, InputStepMode, VertexAttribute, VertexBufferLayout, VertexFormat};

const DEFAULT_VERTEX_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: size_of::<Vertex2>() as BufferAddress,
    step_mode: InputStepMode::Vertex,
    attributes: &[
        VertexAttribute {
            offset: 0,
            shader_location: 0, // corresponds to layout(location = 0) in shader
            format: VertexFormat::Float32x3,
        },
        VertexAttribute {
            offset: size_of::<[f32; 3]>() as BufferAddress,
            shader_location: 1,
            format: VertexFormat::Float32x2,
        },
    ],
};

type V2 = Vector2<f32>;
type V3 = Vector3<f32>;
type V4 = Vector4<f32>;
type Mat4 = Matrix4<f32>;

pub struct Mesh2 {
    vertices: Vec<Vertex2>,
    indices: Vec<u32>,
}

impl Mesh2 {
    pub fn new_from_vertex2(vertices: &Vec<Vertex2>, indices: &Vec<u32>) -> Self {
        Self {
            vertices: vertices.clone(),
            indices: indices.clone(),
        }
    }
    pub fn new(
        vertices: &Vec<V3>,
        indices: &Vec<u32>,
        attribs_2D: &Vec<V2>,
        transform_matrix: Option<Mat4>,
    ) -> Self {
        assert_eq!(vertices.len(), attribs_2D.len());
        let transform_matrix = if let Some(transform_mat) = transform_matrix {
            transform_mat * OPENGL_TO_WGPU_MATRIX
        } else {
            OPENGL_TO_WGPU_MATRIX
        };
        let vertices: Vec<Vertex2> = vertices
            .par_iter()
            .map(|v| {
                let v = transform_matrix * V4::new(v.x, v.y, v.z, 1.0);
                v.xyz() / v.w
            })
            .zip(attribs_2D)
            .map(|(v, a)| Vertex2 {
                position: [v.x, v.y, v.z],
                attrib: [a.x, a.y],
            })
            .collect();
        return Self::new_from_vertex2(&vertices, indices);
    }
}

impl Geometry for Mesh2 {
    fn vertex_desc(&self) -> VertexBufferLayout {
        DEFAULT_VERTEX_LAYOUT
    }

    fn get_vertex_raw(&self) -> &[u8] {
        bytemuck::cast_slice(self.vertices.as_slice())
    }

    fn get_index_raw(&self) -> &[u8] {
        bytemuck::cast_slice(self.indices.as_slice())
    }

    fn get_num_indices(&self) -> usize {
        self.indices.len()
    }
}

pub struct Mesh3 {
    vertices: Vec<Vertex3>,
    indices: Vec<u32>,
}

impl Mesh3 {
    pub fn new_from_vertex3(vertices: &Vec<Vertex3>, indices: &Vec<u32>) -> Self {
        Self {
            vertices: vertices.clone(),
            indices: indices.clone(),
        }
    }
    pub fn new(
        vertices: &Vec<V3>,
        indices: &Vec<u32>,
        attribs_3D: &Vec<V3>,
        transform_matrix: Option<Mat4>,
    ) -> Self {
        assert_eq!(vertices.len(), attribs_3D.len());
        let transform_matrix = if let Some(transform_mat) = transform_matrix {
            transform_mat * OPENGL_TO_WGPU_MATRIX
        } else {
            OPENGL_TO_WGPU_MATRIX
        };
        let vertices: Vec<Vertex3> = vertices
            .par_iter()
            .map(|v| {
                let v = transform_matrix * V4::new(v.x, v.y, v.z, 1.0);
                v.xyz() / v.w
            })
            .zip(attribs_3D)
            .map(|(v, a)| Vertex3 {
                position: [v.x, v.y, v.z],
                attrib: [a.x, a.y, a.z],
            })
            .collect();
        return Self::new_from_vertex3(&vertices, indices);
    }
}

impl Geometry for Mesh3 {
    fn vertex_desc(&self) -> VertexBufferLayout {
        VertexBufferLayout {
            array_stride: size_of::<Vertex3>() as BufferAddress,
            step_mode: InputStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0, // corresponds to layout(location = 0) in shader
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: size_of::<[f32; 3]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }

    fn get_vertex_raw(&self) -> &[u8] {
        bytemuck::cast_slice(self.vertices.as_slice())
    }

    fn get_index_raw(&self) -> &[u8] {
        bytemuck::cast_slice(self.indices.as_slice())
    }

    fn get_num_indices(&self) -> usize {
        self.indices.len()
    }
}

pub struct Pentagon;

impl Pentagon {
    const VERTICES: &'static [Vertex2] = &[
        Vertex2 {
            position: [-0.0868241, 0.49240386, 0.0],
            attrib: [0.4131759, 0.00759614],
        },
        Vertex2 {
            position: [-0.49513406, 0.06958647, 0.0],
            attrib: [0.0048659444, 0.43041354],
        },
        Vertex2 {
            position: [-0.21918549, -0.44939706, 0.0],
            attrib: [0.28081453, 0.949397057],
        },
        Vertex2 {
            position: [0.35966998, -0.3473291, 0.0],
            attrib: [0.85967, 0.84732911],
        },
        Vertex2 {
            position: [0.44147372, 0.2347359, 0.0],
            attrib: [0.9414737, 0.2652641],
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

pub struct Rectangle {
    mesh: Mesh2,
}

impl Rectangle {
    const INDICES: &'static [u32] = &[0, 1, 2, 0, 2, 3];

    pub fn new() -> Self {
        let pos = vec![
            V3::new(0.0, 0.0, 0.0),
            V3::new(1.0, 0.0, 0.0),
            V3::new(1.0, 1.0, 0.0),
            V3::new(0.0, 1.0, 0.0),
        ];
        let attribs = vec![
            V2::new(0.0, 1.0),
            V2::new(1.0, 1.0),
            V2::new(1.0, 0.0),
            V2::new(0.0, 0.0),
        ];
        let indices = Self::INDICES.to_vec();
        Self {
            mesh: Mesh2::new(&pos, &indices, &attribs, None),
        }
    }
}

impl Geometry for Rectangle {
    fn vertex_desc(&self) -> VertexBufferLayout {
        self.mesh.vertex_desc()
    }

    fn get_vertex_raw(&self) -> &[u8] {
        self.mesh.get_vertex_raw()
    }

    fn get_index_raw(&self) -> &[u8] {
        self.mesh.get_index_raw()
    }

    fn get_num_indices(&self) -> usize {
        self.mesh.get_num_indices()
    }
}

pub struct Cube;

impl Cube {
    const SIDE: f32 = 0.5;
    const VERTICES: &'static [Vertex2] = &[
        // 4 vertices on z = 0.5
        Vertex2 {
            position: [-Self::SIDE, -Self::SIDE, Self::SIDE],
            attrib: [0.0, 0.0],
        },
        Vertex2 {
            position: [Self::SIDE, -Self::SIDE, Self::SIDE],
            attrib: [0.0, 1.0],
        },
        Vertex2 {
            position: [Self::SIDE, Self::SIDE, Self::SIDE],
            attrib: [1.0, 0.0],
        },
        Vertex2 {
            position: [-Self::SIDE, Self::SIDE, Self::SIDE],
            attrib: [1.0, 1.0],
        },
        // 4 vertices on z = -0.5
        Vertex2 {
            position: [-Self::SIDE, -Self::SIDE, -Self::SIDE],
            attrib: [0.0, 0.0],
        },
        Vertex2 {
            position: [Self::SIDE, -Self::SIDE, -Self::SIDE],
            attrib: [0.0, 1.0],
        },
        Vertex2 {
            position: [Self::SIDE, Self::SIDE, -Self::SIDE],
            attrib: [1.0, 0.0],
        },
        Vertex2 {
            position: [-Self::SIDE, Self::SIDE, -Self::SIDE],
            attrib: [1.0, 1.0],
        },
    ];

    #[rustfmt::skip]
    const INDICES: &'static [u16] = &[
        0, 1, 3, 3, 1, 2,
        2, 1, 5, 2, 5, 6,
        3, 2, 7, 7, 2, 6,
        4, 0, 3, 4, 3, 7,
        4, 1, 0, 4, 5, 1,
        7, 6, 5, 7, 5, 4
    ];
}

impl Geometry for Cube {
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
