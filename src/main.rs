use cgmath::Matrix4;
use futures::executor::block_on;
use half::f16;
use rayon::prelude::*;
use std::num::NonZeroU32;
use wenderer::rendering::{Camera, CanvasPass, D3Pass, RenderPass};
use wenderer::shading::Tex;
use wenderer::utils::{load_volume_data, CameraController};
use wgpu::{Extent3d, TextureFormat, SurfaceConfiguration, TextureUsages, TextureViewDescriptor, TextureViewDimension, CompositeAlphaMode};
use winit::dpi::PhysicalSize;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct State {
    surface: wgpu::Surface,
    surface_configs: SurfaceConfiguration,
    surface_view_desc: TextureViewDescriptor<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: PhysicalSize<u32>,
    camera: Camera,
    camera_controller: CameraController,
    cube_scaling: Matrix4<f32>,
    front_face_pass: D3Pass,
    front_face_render_buffer: Tex,
    back_face_pass: D3Pass,
    back_face_render_buffer: Tex,
    canvas_pass: CanvasPass,
}

impl State {
    /// This is 1 because render buffer textures for front-face and back-face rendering is the resolved target
    /// not the multisampled target
    const FACE_RENDER_BUFFER_SAMPLE_COUNT: u32 = 1;
    // need async because we need to await some struct creation here
    async fn new(window: &Window, sample_count: NonZeroU32) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        // need adapter to create the device and queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(), //The device you have limits the features you can use
                    limits: wgpu::Limits::default(), //The limits field describes the limit of certain types of resource we can create
                    label: None,
                },
                None,
            )
            .await
            .unwrap();
        let preferred_format = surface.get_supported_formats(&adapter)[0];
        let surface_configs = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: preferred_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto
        };
        surface.configure(&device, &surface_configs);
        let surface_view_desc = TextureViewDescriptor {
            label: Some("Render Texture View"),
            format: Some(surface_configs.format),
            dimension: Some(TextureViewDimension::D2),
            aspect: Default::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };
        // rendering configurations
        let camera = Camera {
            eye: (0.0, -2.5, 1.0).into(),
            center: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_z(),
            aspect: (size.width as f32) / (size.height as f32),
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        // load volume into textures
        let ((x, y, z), data, _uint_data) = load_volume_data("./data/stagbeetle277x277x164.dat");
        let data_f16: Vec<f16> = data.into_par_iter().map(f16::from_f32).collect();
        let extent = Extent3d {
            width: x as u32,
            height: y as u32,
            depth_or_array_layers: z as u32,
        };
        // prepare volume cube scaling for correct shape
        let mut dims = vec![x, y, z];
        dims.sort();
        let mid_val = *dims.get(1).unwrap() as f32;
        let volume_texture =
            Tex::create_3d_texture_red_f16(&extent, &data_f16, &device, &queue, "Volume");
        let cube_scaling = Matrix4::from_nonuniform_scale(
            x as f32 / mid_val,
            y as f32 / mid_val,
            z as f32 / mid_val,
        );

        // prepare front-face and back-face passes
        let face_buffer_format = TextureFormat::Rgba16Float; // filterable format with highest precision
        let front_face_render_buffer = Tex::create_render_buffer(
            (size.width, size.height),
            &device,
            Some("Front face render buffer texture"),
            NonZeroU32::new(State::FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &face_buffer_format,
        );
        let front_face_pass = D3Pass::new(
            &device,
            size.width,
            size.height,
            &front_face_render_buffer.format,
            true,
            &camera,
            sample_count.clone(),
            cube_scaling.clone(),
        );
        let back_face_render_buffer = Tex::create_render_buffer(
            (size.width, size.height),
            &device,
            Some("Back face render buffer texture"),
            NonZeroU32::new(State::FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &face_buffer_format,
        );
        let back_face_pass = D3Pass::new(
            &device,
            size.width,
            size.height,
            &back_face_render_buffer.format,
            false,
            &camera,
            sample_count.clone(),
            cube_scaling.clone(),
        );
        let canvas_pass = CanvasPass::new(
            &front_face_render_buffer,
            &back_face_render_buffer,
            &volume_texture,
            &device,
            &queue,
            (size.width, size.height),
            &preferred_format,
            sample_count,
        );
        Self {
            surface,
            surface_configs,
            surface_view_desc,
            device,
            queue,
            size,
            camera,
            camera_controller: CameraController::new(0.2),
            cube_scaling,
            front_face_pass,
            back_face_pass,
            front_face_render_buffer,
            back_face_render_buffer,
            canvas_pass,
        }
    }
    // If we want to support resizing in our application, we're going to need to recreate the swap_chain everytime the window's size changes.
    // That's the reason we stored the physical size and the sc_desc used to create the swap chain.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.surface_configs.width = new_size.width;
        self.surface_configs.height = new_size.height;

        self.camera.aspect = self.size.width as f32 / self.size.height as f32;
        self.surface.configure(&self.device, &self.surface_configs);
        self.front_face_pass
            .resize(&self.device, self.size.width, self.size.height);
        self.back_face_pass
            .resize(&self.device, self.size.width, self.size.height);
        self.front_face_pass.update_model_view_proj_uniform(
            self.cube_scaling.clone(),
            &self.camera,
            &self.queue,
        );
        self.back_face_pass.update_model_view_proj_uniform(
            self.cube_scaling.clone(),
            &self.camera,
            &self.queue,
        );
        self.canvas_pass
            .resize(&self.device, self.size.width, self.size.height);

        self.front_face_render_buffer = Tex::create_render_buffer(
            (self.size.width, self.size.height),
            &self.device,
            Some("Front Face Render Buffer"),
            NonZeroU32::new(State::FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &self.front_face_render_buffer.format,
        );
        self.back_face_render_buffer = Tex::create_render_buffer(
            (self.size.width, self.size.height),
            &self.device,
            Some("Back Face Render Buffer"),
            NonZeroU32::new(State::FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &self.back_face_render_buffer.format,
        );
        self.canvas_pass.change_bound_face_textures(
            &self.device,
            &self.front_face_render_buffer,
            &self.back_face_render_buffer,
        );
    }
    // input() returns a bool to indicate whether an event has been fully processed.
    // If the method returns true, the main loop won't process the event any further.
    fn input(&mut self, event: &WindowEvent) -> bool {
        if self.camera_controller.process_events(event) {
            return true;
        }
        match event {
            // WindowEvent::CursorMoved { position, .. } => {
            //     if position.x > (self.size.width / 2) as f64 {
            //         self.color_pass.clear_color = (0.3, 0.2, 0.1, 1.0);
            //     } else {
            //         self.color_pass.clear_color = (0.1, 0.2, 0.3, 1.0);
            //     }
            //     return true;
            // }
            _ => {}
        }
        return false;
    }
    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.front_face_pass.update_model_view_proj_uniform(
            self.cube_scaling.clone(),
            &self.camera,
            &self.queue,
        );
        self.back_face_pass.update_model_view_proj_uniform(
            self.cube_scaling.clone(),
            &self.camera,
            &self.queue,
        );
    }
    // We also need to create a CommandEncoder to create the actual commands to send to the gpu.
    // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the gpu.
    // The encoder builds a command buffer that we can then send to the gpu.
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let frame_tex_view = frame.texture.create_view(&self.surface_view_desc);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.front_face_pass
            .render(&self.front_face_render_buffer.view, None, &mut encoder);
        self.back_face_pass
            .render(&self.back_face_render_buffer.view, None, &mut encoder);
        self.canvas_pass.render(&frame_tex_view, None, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1000, 1000))
        .with_title("WebGPU-based DVR")
        .build(&event_loop)
        .unwrap();
    let sample_count = 4;
    let mut state = block_on(State::new(&window, NonZeroU32::new(sample_count).unwrap()));

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            // see the explanation of State::input()
            if !state.input(event) {
                match event {
                    WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Recreate the swap_chain if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("Some unhandled error {:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {}
    })
}
