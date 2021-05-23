use crate::rendering::Camera;
use bytemuck::{Pod, Zeroable};
use cgmath::SquareMatrix;
use crevice::std140::AsStd140;
use mint::ColumnMatrix4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, AsStd140)]
pub struct Uniforms {
    view_proj: ColumnMatrix4<f32>,
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
