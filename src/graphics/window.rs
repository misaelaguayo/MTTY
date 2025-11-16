use crate::{
    config::Config,
    graphics::lib::{Vertex, VERTICES},
    grid::Grid,
};
use font_kit::source::SystemSource;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu_text::{
    glyph_brush::{ab_glyph::FontVec, Section, Text},
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
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

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

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let num_vertices = VERTICES.len() as u32;

        let state = State {
            config: config.clone(),
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            vertex_buffer,
            num_vertices,
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
        let section = Section {
            screen_position: (100.0, 30.0),
            ..Section::default()
        }
        .add_text(
            Text::new("Hello, wgpu_text!")
                .with_scale(40.0)
                .with_color([1.0, 1.0, 0.0, 1.0]),
        );
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
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.draw(0..self.num_vertices, 0..1);
        self.brush
            .queue(&self.device, &self.queue, [&section])
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
                        .with_inner_size(Size::Logical(winit::dpi::LogicalSize {
                            width: self.config.width as f64,
                            height: self.config.height as f64,
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
