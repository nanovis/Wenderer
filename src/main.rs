use std::num::NonZeroU32;
use std::sync::Arc;

use cgmath::Matrix4;
use futures::executor::block_on;
use half::f16;
use rayon::prelude::*;
use wgpu::{CompositeAlphaMode, Extent3d, MemoryHints, SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, TextureViewDimension};
use winit::{
    event::*,
    event_loop::EventLoop,
    window::Window,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey::Code;
use winit::window::WindowId;

use wenderer::rendering::{Camera, CanvasPass, D3Pass, RenderPass};
use wenderer::shading::Tex;
use wenderer::utils::{CameraController, load_volume_data};

/// This is 1 because render buffer textures for front-face and back-face rendering is the resolved target
/// not the multisampled target
const FACE_RENDER_BUFFER_SAMPLE_COUNT: u32 = 1;

struct RenderConfigs {
    sample_count: NonZeroU32,
}

struct RenderState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
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

impl RenderState {
    async fn new(window: Arc<Window>, sample_count: NonZeroU32) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");
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
                    label: None,
                    required_features: wgpu::Features::empty(), //The device you have limits the features you can use
                    required_limits: wgpu::Limits::default(), //The limits field describes the limit of certain types of resource we can create
                    memory_hints: MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();
        let preferred_format = surface.get_capabilities(&adapter).formats[0];
        let surface_configs = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: preferred_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2, // 2 is the default value
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![preferred_format],
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
            NonZeroU32::new(FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
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
            NonZeroU32::new(FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
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
            window,
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
            front_face_render_buffer,
            back_face_pass,
            back_face_render_buffer,
            canvas_pass,
        }
    }
}

struct App {
    render_configs: RenderConfigs,
    render_state: Option<RenderState>,
    window_size: PhysicalSize<u32>,
    title: String,
}

impl App {
    // need async because we need to await some struct creation here
    fn new(render_configs: RenderConfigs,
           window_size: PhysicalSize<u32>,
           title: String) -> Self {
        Self {
            render_configs,
            render_state: None,
            window_size,
            title,
        }
    }

    // If we want to support resizing in our application, we're going to need to recreate the swap_chain everytime the window's size changes.
    // That's the reason we stored the physical size and the sc_desc used to create the swap chain.
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let rs = self.render_state.as_mut().unwrap();
        rs.size = new_size;
        rs.surface_configs.width = new_size.width;
        rs.surface_configs.height = new_size.height;

        rs.camera.aspect = rs.size.width as f32 / rs.size.height as f32;
        rs.surface.configure(&rs.device, &rs.surface_configs);
        rs.front_face_pass
            .resize(&rs.device, rs.size.width, rs.size.height);
        rs.back_face_pass
            .resize(&rs.device, rs.size.width, rs.size.height);
        rs.front_face_pass.update_model_view_proj_uniform(
            rs.cube_scaling.clone(),
            &rs.camera,
            &rs.queue,
        );
        rs.back_face_pass.update_model_view_proj_uniform(
            rs.cube_scaling.clone(),
            &rs.camera,
            &rs.queue,
        );
        rs.canvas_pass
            .resize(&rs.device, rs.size.width, rs.size.height);

        rs.front_face_render_buffer = Tex::create_render_buffer(
            (rs.size.width, rs.size.height),
            &rs.device,
            Some("Front Face Render Buffer"),
            NonZeroU32::new(FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &rs.front_face_render_buffer.format,
        );
        rs.back_face_render_buffer = Tex::create_render_buffer(
            (rs.size.width, rs.size.height),
            &rs.device,
            Some("Back Face Render Buffer"),
            NonZeroU32::new(FACE_RENDER_BUFFER_SAMPLE_COUNT).unwrap(),
            &rs.back_face_render_buffer.format,
        );
        rs.canvas_pass.change_bound_face_textures(
            &rs.device,
            &rs.front_face_render_buffer,
            &rs.back_face_render_buffer,
        );
    }
    // input() returns a bool to indicate whether an event has been fully processed.
    // If the method returns true, the main loop won't process the event any further.
    fn input(&mut self, event: &KeyEvent) -> bool {
        self.render_state.as_mut().unwrap().camera_controller.process_events(event)
    }

    fn update(&mut self) {
        let rs = self.render_state.as_mut().unwrap();
        rs.camera_controller.update_camera(&mut rs.camera);
        rs.front_face_pass.update_model_view_proj_uniform(
            rs.cube_scaling.clone(),
            &rs.camera,
            &rs.queue,
        );
        rs.back_face_pass.update_model_view_proj_uniform(
            rs.cube_scaling.clone(),
            &rs.camera,
            &rs.queue,
        );
    }
    // We also need to create a CommandEncoder to create the actual commands to send to the gpu.
    // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the gpu.
    // The encoder builds a command buffer that we can then send to the gpu.
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let render_state = self.render_state.as_mut().unwrap();
        let frame = render_state.surface.get_current_texture()?;
        let frame_tex_view = frame.texture.create_view(&render_state.surface_view_desc);
        let mut encoder = render_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        render_state.front_face_pass
            .render(&render_state.front_face_render_buffer.view, None, &mut encoder);
        render_state.back_face_pass
            .render(&render_state.back_face_render_buffer.view, None, &mut encoder);
        render_state.canvas_pass.render(&frame_tex_view, None, &mut encoder);
        render_state.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed");
        let window_attributes = Window::default_attributes()
            .with_inner_size(self.window_size)
            .with_title(self.title.clone());
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.render_state = Some(block_on(RenderState::new(window.clone(), self.render_configs.sample_count)));
        // to trigger the first render
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let window = self.render_state.as_ref().unwrap().window.clone();
        if window.id() != window_id {
            return;
        }
        match &event {
            WindowEvent::Resized(physical_size) => self.resize(*physical_size),
            WindowEvent::ScaleFactorChanged { .. } => {
                self.resize(window.inner_size());
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if self.input(event) {
                    window.request_redraw();
                    return;
                }
                if event.state.is_pressed() {
                    match event.physical_key {
                        Code(KeyCode::Escape) => {
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.update();
                match self.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.render_state.as_ref().unwrap().size),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("Some unhandled error {:?}", e),
                }
            }
            _ => {}
        }
    }
}


fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let render_configs = RenderConfigs {
        sample_count: NonZeroU32::new(4).unwrap(),
    };
    let mut app = App::new(render_configs,
                           PhysicalSize::new(1000, 1000),
                           "WebGPU-based DVR".to_string());
    event_loop.run_app(&mut app).expect("Failed to run app");
}
