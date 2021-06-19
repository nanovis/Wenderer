use crate::data::{Vertex2, Vertex3};
use crate::rendering::{Geometry, OPENGL_TO_WGPU_MATRIX};
use cgmath::{Matrix4, Vector2, Vector3, Vector4};
use rayon::prelude::*;
use std::mem::size_of;
use wgpu::{
    BufferAddress, IndexFormat, InputStepMode, VertexAttribute, VertexBufferLayout, VertexFormat,
};

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

pub type V2 = Vector2<f32>;
pub type V3 = Vector3<f32>;
pub type V4 = Vector4<f32>;
pub type Mat4 = Matrix4<f32>;

pub struct Mesh2 {
    vertices: Vec<Vertex2>,
    indices_u16: Vec<u16>,
    indices_u32: Vec<u32>,
    index_format: IndexFormat,
    index_length: usize,
}

impl Mesh2 {
    pub fn new(
        vertices: &Vec<V3>,
        indices: &Vec<usize>,
        attribs_2d: &Vec<V2>,
        transform_matrix: Option<Mat4>,
    ) -> Self {
        assert_eq!(vertices.len(), attribs_2d.len());
        let vertices: Vec<Vertex2> = match transform_matrix {
            Some(transform_mat) => vertices
                .par_iter()
                .map(|v| {
                    let v = transform_mat * V4::new(v.x, v.y, v.z, 1.0);
                    v.xyz() / v.w
                })
                .zip(attribs_2d)
                .map(|(v, a)| Vertex2 {
                    position: [v.x, v.y, v.z],
                    attrib: [a.x, a.y],
                })
                .collect(),
            None => vertices
                .par_iter()
                .zip(attribs_2d)
                .map(|(v, a)| Vertex2 {
                    position: [v.x, v.y, v.z],
                    attrib: [a.x, a.y],
                })
                .collect(),
        };
        let index_length = indices.len();
        if vertices.len() <= u16::MAX as usize {
            let indices = indices.iter().map(|x| *x as u16).collect();
            return Self {
                vertices: vertices.clone(),
                indices_u32: vec![],
                indices_u16: indices,
                index_format: IndexFormat::Uint16,
                index_length,
            };
        } else {
            let indices = indices.iter().map(|x| *x as u32).collect();
            return Self {
                vertices: vertices.clone(),
                indices_u32: indices,
                indices_u16: vec![],
                index_format: IndexFormat::Uint32,
                index_length,
            };
        }
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
        match self.index_format {
            IndexFormat::Uint16 => bytemuck::cast_slice(self.indices_u16.as_slice()),
            IndexFormat::Uint32 => bytemuck::cast_slice(self.indices_u32.as_slice()),
        }
    }

    #[inline]
    fn get_index_format(&self) -> IndexFormat {
        self.index_format
    }

    #[inline]
    fn get_num_indices(&self) -> usize {
        self.index_length
    }
}

pub struct Mesh3 {
    vertices: Vec<Vertex3>,
    indices_u32: Vec<u32>,
    indices_u16: Vec<u16>,
    index_format: IndexFormat,
    index_length: usize,
}

impl Mesh3 {
    pub fn new(
        vertices: &Vec<V3>,
        indices: &Vec<usize>,
        attribs_3d: &Vec<V3>,
        transform_matrix: Option<Mat4>,
    ) -> Self {
        assert_eq!(vertices.len(), attribs_3d.len());
        let vertices: Vec<Vertex3> = match transform_matrix {
            Some(transform_mat) => vertices
                .par_iter()
                .map(|v| {
                    let v = transform_mat * V4::new(v.x, v.y, v.z, 1.0);
                    v.xyz() / v.w
                })
                .zip(attribs_3d)
                .map(|(v, a)| Vertex3 {
                    position: [v.x, v.y, v.z],
                    attrib: [a.x, a.y, a.z],
                })
                .collect(),
            None => vertices
                .par_iter()
                .zip(attribs_3d)
                .map(|(v, a)| Vertex3 {
                    position: [v.x, v.y, v.z],
                    attrib: [a.x, a.y, a.z],
                })
                .collect(),
        };
        let index_length = indices.len();
        if vertices.len() <= u16::MAX as usize {
            let indices = indices.iter().map(|x| *x as u16).collect();
            return Self {
                vertices: vertices.clone(),
                indices_u32: vec![],
                indices_u16: indices,
                index_format: IndexFormat::Uint16,
                index_length,
            };
        } else {
            let indices = indices.iter().map(|x| *x as u32).collect();
            return Self {
                vertices: vertices.clone(),
                indices_u32: indices,
                indices_u16: vec![],
                index_format: IndexFormat::Uint32,
                index_length,
            };
        }
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
        match self.index_format {
            IndexFormat::Uint16 => bytemuck::cast_slice(self.indices_u16.as_slice()),
            IndexFormat::Uint32 => bytemuck::cast_slice(self.indices_u32.as_slice()),
        }
    }

    #[inline]
    fn get_index_format(&self) -> IndexFormat {
        self.index_format
    }

    #[inline]
    fn get_num_indices(&self) -> usize {
        self.index_length
    }
}

pub struct Rectangle {
    mesh: Mesh2,
}

impl Rectangle {
    const INDICES: &'static [usize] = &[0, 1, 2, 0, 2, 3];

    pub fn new_standard_rectangle() -> Self {
        let pos = vec![
            V3::new(-1.0, -1.0, 0.0),
            V3::new(1.0, -1.0, 0.0),
            V3::new(1.0, 1.0, 0.0),
            V3::new(-1.0, 1.0, 0.0),
        ];
        let attribs = vec![
            V2::new(0.0, 1.0),
            V2::new(1.0, 1.0),
            V2::new(1.0, 0.0),
            V2::new(0.0, 0.0),
        ];
        let indices = Self::INDICES.to_vec();
        Self {
            mesh: Mesh2::new(&pos, &indices, &attribs, Some(OPENGL_TO_WGPU_MATRIX)),
        }
    }

    pub fn new_unit_rectangle() -> Self {
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
            mesh: Mesh2::new(&pos, &indices, &attribs, Some(OPENGL_TO_WGPU_MATRIX)),
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

    fn get_index_format(&self) -> IndexFormat {
        self.mesh.get_index_format()
    }

    fn get_num_indices(&self) -> usize {
        self.mesh.get_num_indices()
    }
}
