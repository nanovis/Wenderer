use crate::geometries::{Mesh3, V3};
use crate::rendering::Camera;
use rayon::prelude::*;
use std::iter::FromIterator;
use std::path::Path;
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed; // when the key is released, *state will be Release and thus reset the corresponding state
                match keycode {
                    VirtualKeyCode::Space => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::LShift => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::W | VirtualKeyCode::Up => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::S | VirtualKeyCode::Down => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.center - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.center - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye = camera.center - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.center - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

pub fn create_cube_fbo() -> Mesh3 {
    let side = 1.0;
    let side2 = side / 2.0;
    let vertices = vec![
        // 4 vertices on z = 0.5
        V3::new(-side2, -side2, side2),
        V3::new(side2, -side2, side2),
        V3::new(side2, side2, side2),
        V3::new(-side2, side2, side2),
        // 4 vertices on z = -0.5
        V3::new(-side2, -side2, -side2),
        V3::new(side2, -side2, -side2),
        V3::new(side2, side2, -side2),
        V3::new(-side2, side2, -side2),
    ];
    let attribs_3d = vec![
        // attributes of 4 vertices on z = 0.5
        V3::new(0.0, 0.0, side),
        V3::new(side, 0.0, side),
        V3::new(side, side, side),
        V3::new(0.0, side, side),
        // attributes of 4 vertices on z = 0.5
        V3::new(0.0, 0.0, 0.0),
        V3::new(side, 0.0, 0.0),
        V3::new(side, side, 0.0),
        V3::new(0.0, side, 0.0),
    ];
    #[rustfmt::skip]
    let indices = vec![
        0, 1, 3, 3, 1, 2,
        2, 1, 5, 2, 5, 6,
        3, 2, 7, 7, 2, 6,
        4, 0, 3, 4, 3, 7,
        4, 1, 0, 4, 5, 1,
        7, 6, 5, 7, 5, 4
    ];
    Mesh3::new(&vertices, &indices, &attribs_3d, None)
}

///
/// Reads raw 16-bit data into arrays
///
/// First 3 2-byte unsigned integers should be dimensions
///
/// Following 2-byte integers should only use lower 12bits
///
/// # Returns
/// * dimensions
/// * normalized(data << 4) float array
/// * original u16 data array
///
/// # Endian
/// Native endian of your machine, change `u16::from_ne_bytes` to `u16::from_be_bytes` or `u16::from_le_bytes` if necessary
///
#[cfg(target_arch = "wasm32")]
pub fn load_volume_data<P: AsRef<Path>>(_: P) -> ((usize, usize, usize), Vec<f32>, Vec<u16>) {
    let bytes = include_bytes!("../data/stagbeetle277x277x164.dat");
    let unsigned_shorts: Vec<u16> = bytes
        .par_chunks_exact(2)
        .map(|bytes| u16::from_ne_bytes([bytes[0], bytes[1]]))
        .collect();
    let x = unsigned_shorts.get(0).unwrap().clone() as usize;
    let y = unsigned_shorts.get(1).unwrap().clone() as usize;
    let z = unsigned_shorts.get(2).unwrap().clone() as usize;
    let expected_data_num = x * y * z;
    const U16MAX_F: f32 = u16::MAX as f32;
    let data: Vec<f32> = unsigned_shorts
        .par_iter()
        .skip(3)
        .map(|num| ((*num << 4) as f32) / U16MAX_F)
        .collect();
    let uint_data = Vec::from_iter(unsigned_shorts[3..].iter().cloned());
    assert_eq!(expected_data_num, data.len(), "Data size not match");
    return ((x, y, z), data, uint_data);
}

///
/// Reads raw 16-bit data into arrays
///
/// First 3 2-byte unsigned integers should be dimensions
///
/// Following 2-byte integers should only use lower 12bits
///
/// # Returns
/// * dimensions
/// * normalized(data << 4) float array
/// * original u16 data array
///
/// # Endian
/// Native endian of your machine, change `u16::from_ne_bytes` to `u16::from_be_bytes` or `u16::from_le_bytes` if necessary
///
#[cfg(not(target_arch = "wasm32"))]
pub fn load_volume_data<P: AsRef<Path>>(
    data_path: P,
) -> ((usize, usize, usize), Vec<f32>, Vec<u16>) {
    let bytes = std::fs::read(data_path).expect("Error when reading file");
    let unsigned_shorts: Vec<u16> = bytes
        .par_chunks_exact(2)
        .map(|bytes| u16::from_ne_bytes([bytes[0], bytes[1]]))
        .collect();
    let x = unsigned_shorts.get(0).unwrap().clone() as usize;
    let y = unsigned_shorts.get(1).unwrap().clone() as usize;
    let z = unsigned_shorts.get(2).unwrap().clone() as usize;
    let expected_data_num = x * y * z;
    const U16MAX_F: f32 = u16::MAX as f32;
    let data: Vec<f32> = unsigned_shorts
        .par_iter()
        .skip(3)
        .map(|num| ((*num << 4) as f32) / U16MAX_F)
        .collect();
    let uint_data = Vec::from_iter(unsigned_shorts[3..].iter().cloned());
    assert_eq!(expected_data_num, data.len(), "Data size not match");
    return ((x, y, z), data, uint_data);
}

pub fn load_example_transfer_function() -> Vec<cgmath::Vector4<u8>> {
    #[rustfmt::skip]
    static TF:[f32;48] = [
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.5, 0.5, 0.01,
        0.0, 0.5, 0.5, 0.01,
        0.0, 0.5, 0.5, 0.0,
        0.5, 0.5, 0.0, 0.0,
        0.5, 0.5, 0.0, 0.2,
        0.5, 0.5, 0.0, 0.5,
        0.5, 0.5, 0.0, 0.2,
        0.5, 0.5, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
        1.0, 0.0, 1.0, 0.0,
        1.0, 0.0, 1.0, 0.8
    ];
    TF[..]
        .chunks_exact(4)
        .map(|x| cgmath::Vector4::new(x[0], x[1], x[2], x[3]))
        .map(|v| v * (u8::MAX as f32))
        .map(|v| cgmath::Vector4::new(v.x as u8, v.y as u8, v.z as u8, v.w as u8))
        .collect()
}

#[cfg(test)]
mod util_tests {
    use super::*;
    #[test]
    fn test_load_data() {
        let (_, _, _data) = load_volume_data("./data/stagbeetle277x277x164.dat");
    }
}
