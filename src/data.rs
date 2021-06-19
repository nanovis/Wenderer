use crate::rendering::Camera;
use bytemuck::{Pod, Zeroable};
use cgmath::SquareMatrix;
use crevice::std140::AsStd140;
use mint::ColumnMatrix4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex2 {
    pub position: [f32; 3],
    pub attrib: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex3 {
    pub position: [f32; 3],
    pub attrib: [f32; 3],
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

#[repr(C)]
#[derive(Debug, Copy, Clone, AsStd140)]
pub struct CanvasShaderUniforms {
    pub step_size: f32,
    pub base_distance: f32,
    pub opacity_threshold: f32,
    pub ambient_intensity: f32,
    pub diffuse_intensity: f32,
    pub specular_intensity: f32,
    pub shininess: f32,
}

impl Default for CanvasShaderUniforms {
    fn default() -> Self {
        Self {
            step_size: 0.0025,
            base_distance: 0.0025,
            opacity_threshold: 0.95,
            ambient_intensity: 0.5,
            diffuse_intensity: 0.5,
            specular_intensity: 0.5,
            shininess: 32.0,
        }
    }
}
