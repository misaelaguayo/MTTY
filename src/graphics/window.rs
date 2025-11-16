use crate::{config::Config, grid::Grid};
use font_kit::source::SystemSource;
use std::sync::Arc;
use wgpu_text::{
    glyph_brush::{
        ab_glyph::{FontVec, PxScale},
        Section, Text,
    },
    BrushBuilder, TextBrush,
};
use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::WindowEvent,
    event_loop::{self, ControlFlow, EventLoop},
    window::Window,
};

struct State {
    grid: Grid,
    config: Arc<Config>,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    brush: TextBrush<FontVec>,
}

impl State {
    async fn new(window: Arc<Window>, config: Arc<Config>) -> State {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let surface = instance.create_surface(window.clone()).unwrap();

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let font_bytes = SystemSource::new()
            .select_by_postscript_name("Hack Nerd Font")
            .expect("Failed to load system font")
            .load()
            .expect("Failed to load font data")
            .copy_font_data()
            .expect("Failed to copy font data");

        let font_vec = FontVec::try_from_vec(font_bytes.to_vec()).unwrap();
        let brush = BrushBuilder::using_font(font_vec).build(
            &device,
            size.width,
            size.height,
            surface_format,
        );

        let state = State {
            config: config.clone(),
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            brush,
            grid: Grid::new(config.clone()),
        };

        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.config.width as u32,
            height: self.config.height as u32,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.brush
            .resize_view(new_size.width as f32, new_size.height as f32, &self.queue);
        self.configure_surface();
        self.config = Arc::new(Config {
            width: new_size.width as f32,
            height: new_size.height as f32,
            ..(*self.config).clone()
        });

        self.grid.resize()
    }

    fn render(&mut self) {
        let start_row = self
            .grid
            .scroll_pos
            .saturating_sub(self.grid.height as usize);
        let end_row = self.grid.active_grid().len();

        let mut sections: Vec<Section> = Vec::new();

        for i in start_row..end_row as usize {
            for j in 0..self.grid.width as usize {
                let cell = self.grid.active_grid()[i][j].clone();
                let (y, x) = self.grid.get_cell_pos(i as u16, j as u16);

                let cell_string = Box::leak(cell.char.to_string().into_boxed_str());
                let text = Text::new(cell_string)
                    .with_scale(PxScale {
                        x: self.config.font_size,
                        y: self.config.font_size,
                    })
                    .with_color([1.0, 1.0, 0.0, 1.0]);

                let section = Section {
                    screen_position: (x as f32, y as f32),
                    ..Section::default()
                }
                .add_text(text);

                sections.push(section);
            }
        }

        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.brush
            .queue(&self.device, &self.queue, sections)
            .unwrap();
        self.brush.draw(&mut renderpass);

        drop(renderpass);
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

struct App {
    config: Arc<Config>,
    state: Option<State>,
}

impl App {
    fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("MTTY")
                        .with_inner_size(Size::Physical(winit::dpi::PhysicalSize {
                            width: self.config.width as u32,
                            height: self.config.height as u32,
                        })),
                )
                .unwrap(),
        );
        let state = pollster::block_on(State::new(window.clone(), self.config.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
            }
            _ => (),
        }
    }
}

pub fn display_grid(config: Arc<Config>) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(config);
    event_loop.run_app(&mut app).unwrap();

    Ok(())
}
