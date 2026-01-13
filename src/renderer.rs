use std::sync::Arc;

use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::{
    Backends, Buffer as WgpuBuffer, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PipelineCompilationOptions, PresentMode, Queue, RenderPipeline,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages,
};
use winit::{dpi::PhysicalSize, window::Window};

/// Detect if running under WSL2 by checking for WSL-specific indicators
fn is_wsl2() -> bool {
    // Check for WSL-specific environment variable
    if std::env::var("WSL_DISTRO_NAME").is_ok() {
        return true;
    }

    // Check /proc/version for Microsoft/WSL indicators
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let version_lower = version.to_lowercase();
        if version_lower.contains("microsoft") || version_lower.contains("wsl") {
            return true;
        }
    }

    false
}

use crate::{
    config::Config,
    grid::Grid,
    styles::{Color, Styles},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BgVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl BgVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BgVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct Renderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    size: PhysicalSize<u32>,

    // Text rendering (glyphon)
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    text_buffer: Buffer,

    // Background rendering
    bg_pipeline: RenderPipeline,
    bg_vertex_buffer: WgpuBuffer,
    bg_index_buffer: WgpuBuffer,

    // Cell dimensions
    cell_width: f32,
    cell_height: f32,

    // Font family name (None = system monospace)
    font_family: Option<String>,

    // Per-row cached render data for incremental updates
    cached_row_bg_vertices: Vec<Vec<BgVertex>>,
    cached_row_text_spans: Vec<Vec<(String, GlyphonColor)>>,
    num_cached_rows: usize,
    // Current number of indices for draw call
    current_bg_index_count: u32,
    // Reusable combined buffers to avoid allocations
    combined_bg_vertices: Vec<BgVertex>,
    combined_bg_indices: Vec<u32>,
    combined_text_spans: Vec<(String, GlyphonColor)>,
}

impl Renderer {
    pub fn new(window: Arc<Window>, config: &Config) -> Self {
        let size = window.inner_size();
        let font_size = config.font_size;

        // Create wgpu instance
        // On WSL2, check for display server availability
        if is_wsl2() {
            let display_set = std::env::var("DISPLAY").is_ok()
                && !std::env::var("DISPLAY").unwrap_or_default().is_empty();
            let wayland_set = std::env::var("WAYLAND_DISPLAY").is_ok()
                && !std::env::var("WAYLAND_DISPLAY")
                    .unwrap_or_default()
                    .is_empty();

            if !display_set && !wayland_set {
                log::error!("WSL2 detected but no display server found (DISPLAY and WAYLAND_DISPLAY are unset)");
                log::error!("Please ensure WSLg is enabled: run 'wsl --update' from Windows and restart WSL");
                log::error!("Or set DISPLAY if using an X server like VcXsrv");
                panic!("No display server available. WSL2 requires WSLg or an X server for GUI applications. \
                       Run 'wsl --update' from Windows PowerShell and restart WSL with 'wsl --shutdown'.");
            }
            log::info!(
                "WSL2 detected, DISPLAY={:?}, WAYLAND_DISPLAY={:?}",
                std::env::var("DISPLAY").ok(),
                std::env::var("WAYLAND_DISPLAY").ok()
            );
        }

        // On WSL2, try Vulkan first (native WSLg support), then GL as fallback
        let backends = if is_wsl2() {
            log::info!("WSL2 detected, trying Vulkan and GL backends");
            Backends::VULKAN | Backends::GL
        } else {
            Backends::all()
        };

        let instance = Instance::new(&InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Create surface with better error handling
        let surface = instance.create_surface(window.clone()).unwrap_or_else(|e| {
            log::error!("Failed to create surface: {:?}", e);
            if is_wsl2() {
                panic!(
                    "Surface creation failed on WSL2. Ensure WSLg is properly configured: \
                       1. Run 'wsl --update' from Windows PowerShell \
                       2. Restart WSL with 'wsl --shutdown' \
                       3. Ensure your GPU drivers are up to date on Windows"
                );
            } else {
                panic!("Failed to create rendering surface: {:?}", e);
            }
        });

        // Request adapter and device
        let (adapter, device, queue) = pollster::block_on(async {
            // In WSL2, try with fallback adapter enabled for better compatibility
            // Also try fallback if the primary adapter request fails
            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .or_else(|| {
                    log::warn!("Primary adapter not available, trying fallback adapter");
                    pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: true,
                    }))
                })
                .expect("Failed to find an appropriate adapter. Ensure your graphics drivers are installed and up to date. On WSL2, enable GPU support with 'wsl --update'.");

            log::info!("Using graphics adapter: {:?}", adapter.get_info());

            let (device, queue) = adapter
                .request_device(
                    &DeviceDescriptor {
                        label: Some("MTTY Device"),
                        required_features: Features::empty(),
                        required_limits: Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .expect("Failed to create device");

            (adapter, device, queue)
        });

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Initialize glyphon for text rendering (uses system fonts)
        let mut font_system = FontSystem::new();

        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        let viewport = Viewport::new(&device, &cache);

        // Store font family from config
        let font_family = config.font_family.clone();

        // Create text buffer for rendering
        let line_height = font_size * 1.2;
        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(font_size, line_height));
        text_buffer.set_size(
            &mut font_system,
            Some(size.width as f32),
            Some(size.height as f32),
        );

        // Measure actual cell width from font by shaping a character
        let mut measure_buffer =
            Buffer::new(&mut font_system, Metrics::new(font_size, line_height));
        let font_attrs = match &font_family {
            Some(name) => Attrs::new().family(Family::Name(name)),
            None => Attrs::new().family(Family::Monospace),
        };
        measure_buffer.set_text(&mut font_system, "M", font_attrs, Shaping::Advanced);
        measure_buffer.shape_until_scroll(&mut font_system, false);

        let cell_width = measure_buffer
            .layout_runs()
            .next()
            .and_then(|run| run.glyphs.first())
            .map(|g| g.w)
            .unwrap_or(font_size * 0.6);
        let cell_height = line_height;

        log::info!(
            "Measured cell dimensions: {}x{} (font_size: {}, family: {:?})",
            cell_width,
            cell_height,
            font_size,
            font_family
        );

        // Create background rendering pipeline
        let bg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bg.wgsl").into()),
        });

        let bg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Background Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Background Pipeline"),
            layout: Some(&bg_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &bg_shader,
                entry_point: Some("vs_main"),
                buffers: &[BgVertex::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bg_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
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

        // Pre-allocate buffers for background quads
        // Estimate max cells based on window size
        let max_cells =
            ((size.width as f32 / cell_width) * (size.height as f32 / cell_height)) as usize + 1000;

        let bg_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Background Vertex Buffer"),
            size: (max_cells * 4 * std::mem::size_of::<BgVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Background Index Buffer"),
            size: (max_cells * 6 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            surface,
            surface_config,
            size,
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            viewport,
            text_buffer,
            bg_pipeline,
            bg_vertex_buffer,
            bg_index_buffer,
            cell_width,
            cell_height,
            font_family,
            cached_row_bg_vertices: Vec::new(),
            cached_row_text_spans: Vec::new(),
            num_cached_rows: 0,
            current_bg_index_count: 0,
            combined_bg_vertices: Vec::with_capacity(max_cells * 4),
            combined_bg_indices: Vec::with_capacity(max_cells * 6),
            combined_text_spans: Vec::with_capacity(1000),
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);

            // Update text buffer size
            self.text_buffer.set_size(
                &mut self.font_system,
                Some(new_size.width as f32),
                Some(new_size.height as f32),
            );

            // Reallocate background buffers for new size
            let max_cells = ((new_size.width as f32 / self.cell_width)
                * (new_size.height as f32 / self.cell_height)) as usize
                + 1000;

            self.bg_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Background Vertex Buffer"),
                size: (max_cells * 4 * std::mem::size_of::<BgVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            self.bg_index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Background Index Buffer"),
                size: (max_cells * 6 * std::mem::size_of::<u32>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Invalidate row caches on resize
            self.cached_row_bg_vertices.clear();
            self.cached_row_text_spans.clear();
            self.num_cached_rows = 0;
            self.current_bg_index_count = 0;
            self.combined_bg_vertices.clear();
            self.combined_bg_indices.clear();
            self.combined_text_spans.clear();
        }
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn cell_dimensions(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    pub fn render(&mut self, grid: &mut Grid) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Get dirty rows info
        let dirty_rows = grid.dirty_rows();
        let num_visible_rows = grid.height as usize;

        // Check if we need to rebuild (any dirty rows or cache size mismatch)
        let needs_rebuild = grid.is_dirty() || self.num_cached_rows != num_visible_rows;

        if needs_rebuild {
            // Ensure caches are properly sized
            if self.num_cached_rows != num_visible_rows {
                self.cached_row_bg_vertices
                    .resize(num_visible_rows, Vec::new());
                self.cached_row_text_spans
                    .resize(num_visible_rows, Vec::new());
                self.num_cached_rows = num_visible_rows;
            }

            // Build render data only for dirty rows
            self.build_render_data_incremental(grid, dirty_rows);

            // Clear and reuse combined buffers
            self.combined_bg_vertices.clear();
            self.combined_bg_indices.clear();
            self.combined_text_spans.clear();
            let mut vertex_offset = 0u32;

            for row_idx in 0..num_visible_rows {
                // Add background vertices
                self.combined_bg_vertices.extend_from_slice(&self.cached_row_bg_vertices[row_idx]);

                // Each row's indices need to be offset by the current vertex count
                let row_vertex_count = self.cached_row_bg_vertices[row_idx].len() as u32;
                // Generate indices for quads (4 vertices per quad, 6 indices per quad)
                let num_quads = row_vertex_count / 4;
                for quad in 0..num_quads {
                    let base = vertex_offset + quad * 4;
                    self.combined_bg_indices.push(base);
                    self.combined_bg_indices.push(base + 1);
                    self.combined_bg_indices.push(base + 2);
                    self.combined_bg_indices.push(base);
                    self.combined_bg_indices.push(base + 2);
                    self.combined_bg_indices.push(base + 3);
                }
                vertex_offset += row_vertex_count;

                // Add text spans (clone needed for glyphon)
                self.combined_text_spans.extend(self.cached_row_text_spans[row_idx].iter().cloned());
            }

            // Store index count for draw call
            self.current_bg_index_count = self.combined_bg_indices.len() as u32;

            // Upload background data
            if !self.combined_bg_vertices.is_empty() {
                self.queue.write_buffer(
                    &self.bg_vertex_buffer,
                    0,
                    bytemuck::cast_slice(&self.combined_bg_vertices),
                );
                self.queue.write_buffer(
                    &self.bg_index_buffer,
                    0,
                    bytemuck::cast_slice(&self.combined_bg_indices),
                );
            }

            // Prepare text rendering with per-character colors
            let rich_text: Vec<(&str, Attrs)> = self
                .combined_text_spans
                .iter()
                .map(|(text, color)| {
                    let attrs = match &self.font_family {
                        Some(name) => Attrs::new().family(Family::Name(name)).color(*color),
                        None => Attrs::new().family(Family::Monospace).color(*color),
                    };
                    (text.as_str(), attrs)
                })
                .collect();

            let default_attrs = match &self.font_family {
                Some(name) => Attrs::new().family(Family::Name(name)),
                None => Attrs::new().family(Family::Monospace),
            };
            self.text_buffer.set_rich_text(
                &mut self.font_system,
                rich_text,
                default_attrs,
                Shaping::Advanced,
            );

            // Shape the text to calculate glyph positions
            self.text_buffer
                .shape_until_scroll(&mut self.font_system, false);

            // Clear the dirty flag now that we've processed the changes
            grid.clear_dirty();
        }

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.size.width,
                height: self.size.height,
            },
        );

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.viewport,
                [TextArea {
                    buffer: &self.text_buffer,
                    left: 0.0,
                    top: 0.0,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: self.size.width as i32,
                        bottom: self.size.height as i32,
                    },
                    default_color: GlyphonColor::rgb(255, 255, 255),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .unwrap();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Convert default background color to wgpu::Color for clearing
        let default_bg = color_to_rgba(grid.styles.default_background_color, &grid.styles);
        let clear_color = wgpu::Color {
            r: default_bg[0] as f64,
            g: default_bg[1] as f64,
            b: default_bg[2] as f64,
            a: 1.0,
        };

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render backgrounds
            if self.current_bg_index_count > 0 {
                render_pass.set_pipeline(&self.bg_pipeline);
                render_pass.set_vertex_buffer(0, self.bg_vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.bg_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.current_bg_index_count, 0, 0..1);
            }

            // Render text
            self.text_renderer
                .render(&self.text_atlas, &self.viewport, &mut render_pass)
                .unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Trim atlas to free unused memory
        self.text_atlas.trim();

        Ok(())
    }

    /// Build render data incrementally, only updating dirty rows
    fn build_render_data_incremental(&mut self, grid: &Grid, dirty_rows: &[bool]) {
        let styles = &grid.styles;
        let width = self.size.width as f32;
        let height = self.size.height as f32;

        // Get default background for comparison (skip rendering cells that match default)
        let default_bg = color_to_rgba(styles.default_background_color, styles);

        let start_row = grid.scroll_pos.saturating_sub(grid.height as usize - 1);
        let active_cells = grid.active_grid_ref();
        let grid_len = active_cells.len();
        let num_visible_rows = grid.height as usize;

        // Process each visible row
        for display_row in 0..num_visible_rows {
            // Skip rows that aren't dirty
            if display_row < dirty_rows.len() && !dirty_rows[display_row] {
                continue;
            }

            let row_idx = start_row + display_row;

            // Clear and rebuild this row's cached data
            self.cached_row_bg_vertices[display_row].clear();
            self.cached_row_text_spans[display_row].clear();

            // Batch consecutive characters with same color for this row
            let mut current_span = String::new();
            let mut current_color: Option<GlyphonColor> = None;

            for col_idx in 0..grid.width as usize {
                let cell_index = row_idx * grid.width as usize + col_idx;

                // Bounds check to prevent crash on grid corruption
                if cell_index >= grid_len {
                    // Fill rest of row with spaces
                    current_span.push(' ');
                    continue;
                }

                // Get cell from the active grid
                let cell = &active_cells[cell_index];

                // Calculate cell position in pixels
                let x = col_idx as f32 * self.cell_width;
                let y = display_row as f32 * self.cell_height;

                // Get background color
                let bg_color = color_to_rgba(cell.bg, styles);

                // Only render backgrounds that differ from the default (optimization)
                let colors_differ = (bg_color[0] - default_bg[0]).abs() > 0.01
                    || (bg_color[1] - default_bg[1]).abs() > 0.01
                    || (bg_color[2] - default_bg[2]).abs() > 0.01;
                if colors_differ {
                    // Convert to normalized device coordinates (-1 to 1)
                    let x0 = (x / width) * 2.0 - 1.0;
                    let y0 = 1.0 - (y / height) * 2.0;
                    let x1 = ((x + self.cell_width) / width) * 2.0 - 1.0;
                    let y1 = 1.0 - ((y + self.cell_height) / height) * 2.0;

                    self.cached_row_bg_vertices[display_row].push(BgVertex {
                        position: [x0, y0],
                        color: bg_color,
                    });
                    self.cached_row_bg_vertices[display_row].push(BgVertex {
                        position: [x1, y0],
                        color: bg_color,
                    });
                    self.cached_row_bg_vertices[display_row].push(BgVertex {
                        position: [x1, y1],
                        color: bg_color,
                    });
                    self.cached_row_bg_vertices[display_row].push(BgVertex {
                        position: [x0, y1],
                        color: bg_color,
                    });
                }

                // Build text content - handle cursor
                let char_to_render = if row_idx == grid.cursor_pos.0 && col_idx == grid.cursor_pos.1
                {
                    styles
                        .cursor_state
                        .to_string()
                        .chars()
                        .next()
                        .unwrap_or(' ')
                } else {
                    cell.char
                };

                // Get foreground color for this cell
                let fg_color = color_to_glyphon(cell.fg, styles);

                // Batch characters with same color
                match current_color {
                    Some(color) if colors_equal(color, fg_color) => {
                        current_span.push(char_to_render);
                    }
                    _ => {
                        // Flush previous span
                        if !current_span.is_empty() {
                            if let Some(color) = current_color {
                                self.cached_row_text_spans[display_row]
                                    .push((std::mem::take(&mut current_span), color));
                            }
                        }
                        current_span.push(char_to_render);
                        current_color = Some(fg_color);
                    }
                }
            }

            // Flush span at end of row
            if !current_span.is_empty() {
                if let Some(color) = current_color {
                    self.cached_row_text_spans[display_row].push((current_span, color));
                }
            }

            // Add newline at end of row
            self.cached_row_text_spans[display_row]
                .push(("\n".to_string(), GlyphonColor::rgb(255, 255, 255)));
        }
    }
}

fn colors_equal(a: GlyphonColor, b: GlyphonColor) -> bool {
    a.r() == b.r() && a.g() == b.g() && a.b() == b.b() && a.a() == b.a()
}

fn color_to_glyphon(color: Color, styles: &Styles) -> GlyphonColor {
    let (r, g, b) = match color {
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::White => (229, 229, 229),
        Color::Gray => (102, 102, 102),
        Color::BrightRed => (241, 76, 76),
        Color::BrightGreen => (35, 209, 139),
        Color::BrightYellow => (245, 245, 67),
        Color::BrightBlue => (59, 142, 234),
        Color::BrightMagenta => (214, 112, 214),
        Color::BrightCyan => (41, 184, 219),
        Color::BrightWhite => (255, 255, 255),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Foreground => {
            return color_to_glyphon(styles.active_text_color, styles);
        }
        Color::Background => {
            return color_to_glyphon(styles.active_background_color, styles);
        }
        Color::ColorIndex(i) => {
            return color_to_glyphon(styles.color_array[i as usize], styles);
        }
    };
    GlyphonColor::rgb(r, g, b)
}

fn color_to_rgba(color: Color, styles: &Styles) -> [f32; 4] {
    let (r, g, b) = match color {
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::White => (229, 229, 229),
        Color::Gray => (102, 102, 102),
        Color::BrightRed => (241, 76, 76),
        Color::BrightGreen => (35, 209, 139),
        Color::BrightYellow => (245, 245, 67),
        Color::BrightBlue => (59, 142, 234),
        Color::BrightMagenta => (214, 112, 214),
        Color::BrightCyan => (41, 184, 219),
        Color::BrightWhite => (255, 255, 255),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Foreground => {
            return color_to_rgba(styles.active_text_color, styles);
        }
        Color::Background => {
            return color_to_rgba(styles.active_background_color, styles);
        }
        Color::ColorIndex(i) => {
            return color_to_rgba(styles.color_array[i as usize], styles);
        }
    };
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}
