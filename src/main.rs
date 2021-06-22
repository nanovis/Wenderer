use futures::executor::block_on;
use std::num::NonZeroU32;
use wenderer::rendering::{Camera, CanvasPass, D3Pass, RenderPass};
use wenderer::shading::Tex;
use wenderer::utils::CameraController;
use winit::dpi::PhysicalSize;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    camera: Camera,
    camera_controller: CameraController,
    front_face_pass: D3Pass,
    front_face_render_buffer: Tex,
    back_face_pass: D3Pass,
    back_face_render_buffer: Tex,
    canvas_pass: CanvasPass,
    sample_count: NonZeroU32,
}

impl State {
    // need async because we need to await some struct creation here
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        // need adapter to create the device and queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
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
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT, //RENDER_ATTACHMENT specifies that the textures will be used to write to the screen
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
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
        let sample_count = NonZeroU32::new(4).unwrap();
        let front_face_render_buffer = Tex::create_render_buffer(
            (size.width, size.height),
            &device,
            Some("Front face render buffer texture"),
            sample_count.clone(),
        );
        let front_face_pass = D3Pass::new(
            &device,
            size.width,
            size.height,
            &front_face_render_buffer.format,
            true,
            &camera,
            sample_count.clone(),
        );
        let back_face_render_buffer = Tex::create_render_buffer(
            (size.width, size.height),
            &device,
            Some("Back face render buffer texture"),
            sample_count.clone(),
        );
        let back_face_pass = D3Pass::new(
            &device,
            size.width,
            size.height,
            &back_face_render_buffer.format,
            false,
            &camera,
            sample_count.clone(),
        );
        let canvas_pass = CanvasPass::new(
            &front_face_render_buffer,
            &back_face_render_buffer,
            &device,
            &queue,
            &sc_desc.format,
            sample_count.clone(),
        );
        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            camera,
            camera_controller: CameraController::new(0.2),
            front_face_pass,
            back_face_pass,
            front_face_render_buffer,
            back_face_render_buffer,
            canvas_pass,
            sample_count,
        }
    }
    // If we want to support resizing in our application, we're going to need to recreate the swap_chain everytime the window's size changes.
    // That's the reason we stored the physical size and the sc_desc used to create the swap chain.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;

        self.camera.aspect = self.size.width as f32 / self.size.height as f32;

        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.front_face_pass
            .resize(&self.device, self.size.width, self.size.height);
        self.back_face_pass
            .resize(&self.device, self.size.width, self.size.height);
        self.front_face_pass
            .update_view_proj_uniform(&self.camera, &self.queue);
        self.back_face_pass
            .update_view_proj_uniform(&self.camera, &self.queue);

        self.front_face_render_buffer = Tex::create_render_buffer(
            (self.size.width, self.size.height),
            &self.device,
            Some("Front Face Render Buffer"),
            self.sample_count.clone(),
        );
        self.back_face_render_buffer = Tex::create_render_buffer(
            (self.size.width, self.size.height),
            &self.device,
            Some("Back Face Render Buffer"),
            self.sample_count.clone(),
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
        self.front_face_pass
            .update_view_proj_uniform(&self.camera, &self.queue);
        self.back_face_pass
            .update_view_proj_uniform(&self.camera, &self.queue);
    }
    // We also need to create a CommandEncoder to create the actual commands to send to the gpu.
    // Most modern graphics frameworks expect commands to be stored in a command buffer before being sent to the gpu.
    // The encoder builds a command buffer that we can then send to the gpu.
    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.front_face_pass
            .render(&self.front_face_render_buffer.view, None, &mut encoder);
        self.back_face_pass
            .render(&self.back_face_render_buffer.view, None, &mut encoder);
        self.canvas_pass.render(&frame.view, None, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
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
    let mut state = block_on(State::new(&window));

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
                Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
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
