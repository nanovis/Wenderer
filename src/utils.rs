use crate::geometries::{Mesh3, V2, V3};
use crate::rendering::Camera;
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
    let attribs_3D = vec![
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
    Mesh3::new(&vertices, &indices, &attribs_3D, None)
}
