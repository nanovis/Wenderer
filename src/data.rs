use crate::rendering::{Camera, Uniform};
use bytemuck::{Pod, Zeroable};
use cgmath::SquareMatrix;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vert<const POS_LENGTH: usize, const ATTRIB_LENGTH: usize> {
    pub position: [f32; POS_LENGTH],
    pub attrib: [f32; ATTRIB_LENGTH],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

impl Uniform for Uniforms {
    fn to_raw_u8(&self) -> &[u8] {
        return bytemuck::cast_slice(&self.view_proj);
    }
}
