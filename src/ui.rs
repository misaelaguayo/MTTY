use std::{
    cmp::{max, min},
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use tokio::sync::broadcast::{Receiver, Sender};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    commands::{ClientCommand, IdentifyTerminalMode, ServerCommand, SgrAttribute},
    config::Config,
    grid::{Cell, Grid},
    renderer::Renderer,
    styles::{Color, Styles},
};

#[cfg(test)]
mod tests;

// Trait defining a runner that can execute the UI
// This allows for different implementations of the UI
pub trait Runner {
    fn run(self);
}

pub struct WgpuRunner {
    pub exit_flag: Arc<AtomicBool>,
    pub config: Config,
    pub tx: Sender<ServerCommand>,
    pub rx: Receiver<ClientCommand>,
}

impl Runner for WgpuRunner {
    fn run(self) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        // Use Wait instead of Poll to reduce CPU usage when idle
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = WgpuApp::new(
            &self.config,
            self.exit_flag.clone(),
            self.tx.clone(),
            self.rx.resubscribe(),
        );

        event_loop.run_app(&mut app).expect("Event loop failed");
    }
}

/// Debounce duration for window resize events to avoid excessive grid/PTY updates
const RESIZE_DEBOUNCE_MS: u64 = 50;

/// Debug information displayed as an overlay
pub struct DebugInfo {
    /// Whether to show debug overlay (toggled with Ctrl+Shift+I)
    pub show: bool,
    /// Last time FPS was calculated
    last_update: Instant,
    /// Frame count since last FPS update
    frame_count: u32,
    /// Current FPS value
    pub fps: f32,
}

impl DebugInfo {
    fn new() -> Self {
        Self {
            show: false,
            last_update: Instant::now(),
            frame_count: 0,
            fps: 0.0,
        }
    }

    fn update(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_update.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_update = Instant::now();
        }
    }
}

pub struct WgpuApp {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<ServerCommand>,
    rx: Receiver<ClientCommand>,
    config: Config,
    grid: Grid,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    modifiers: winit::keyboard::ModifiersState,
    /// Pending resize to be applied after debounce period
    pending_resize: Option<PhysicalSize<u32>>,
    /// Deadline after which the pending resize should be applied
    resize_deadline: Option<Instant>,
    /// Debug overlay information
    debug_info: DebugInfo,
    /// Cursor keys application mode (DECCKM)
    cursor_keys_mode: bool,
    /// Bracketed paste mode
    bracketed_paste_mode: bool,
}

impl WgpuApp {
    pub fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<ServerCommand>,
        rx: Receiver<ClientCommand>,
    ) -> Self {
        log::info!("Grid size: {} x {}", config.rows, config.cols);
        Self {
            exit_flag,
            input: String::new(),
            tx,
            rx,
            config: config.clone(),
            grid: Grid::new(config),
            window: None,
            renderer: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            pending_resize: None,
            resize_deadline: None,
            debug_info: DebugInfo::new(),
            cursor_keys_mode: false,
            bracketed_paste_mode: false,
        }
    }

    fn send_raw_data(&self, data: Vec<u8>) {
        if let Err(e) = self.tx.send(ServerCommand::RawData(data)) {
            log::warn!("Failed to send raw data: {}", e);
        }
    }

    fn handle_sgr_attribute(&mut self, attribute: SgrAttribute) {
        match attribute {
            SgrAttribute::Reset => {
                self.grid.styles = Styles::default();
            }
            SgrAttribute::Bold => {
                self.grid.styles.font_size = 20;
            }
            SgrAttribute::Dim => {
                self.grid.styles.font_size = 14;
            }
            SgrAttribute::Italic => {
                self.grid.styles.italic = true;
            }
            SgrAttribute::Underline => {
                self.grid.styles.underline = true;
            }
            SgrAttribute::DoubleUnderline => {}
            SgrAttribute::Undercurl => {}
            SgrAttribute::DottedUnderline => {}
            SgrAttribute::DashedUnderline => {}
            SgrAttribute::BlinkSlow => {}
            SgrAttribute::BlinkFast => {}
            SgrAttribute::Reverse => {
                self.grid.styles.reverse = true;
            }
            SgrAttribute::Hidden => {}
            SgrAttribute::Strike => {}
            SgrAttribute::CancelBold => {
                self.grid.styles.font_size = 16;
            }
            SgrAttribute::CancelBoldDim => {
                self.grid.styles.font_size = 16;
            }
            SgrAttribute::CancelItalic => {
                self.grid.styles.italic = false;
            }
            SgrAttribute::CancelUnderline => {
                self.grid.styles.underline = false;
            }
            SgrAttribute::CancelBlink => {}
            SgrAttribute::CancelReverse => {
                self.grid.styles.reverse = false;
            }
            SgrAttribute::CancelHidden => {}
            SgrAttribute::Foreground(color) => match color {
                Color::Foreground => {
                    self.grid.styles.active_text_color = self.grid.styles.default_text_color
                }
                Color::Background => {
                    self.grid.styles.active_text_color = self.grid.styles.default_background_color
                }
                _ => {
                    self.grid.styles.active_text_color = color;
                }
            },
            SgrAttribute::Background(color) => match color {
                Color::Foreground => {
                    self.grid.styles.active_background_color = self.grid.styles.default_text_color
                }
                Color::Background => {
                    self.grid.styles.active_background_color =
                        self.grid.styles.default_background_color
                }
                _ => {
                    self.grid.styles.active_background_color = color;
                }
            },
            _ => {}
        }
    }

    fn handle_command(&mut self, command: ClientCommand) {
        let cols = self.grid.width;
        match command {
            ClientCommand::Backspace => {
                self.grid.delete_character();
            }
            ClientCommand::Print(c) => {
                self.grid.place_character_in_grid(cols, c);
            }
            ClientCommand::NewLine => {
                self.grid.place_character_in_grid(cols, '\n');
            }
            ClientCommand::CarriageReturn => {
                self.grid.place_character_in_grid(cols, '\r');
            }
            ClientCommand::LineFeed => {
                self.grid.set_pos(self.grid.cursor_pos.0 + 1, 0);
            }
            ClientCommand::ClearScreen => {
                self.grid.clear_screen();
            }
            ClientCommand::MoveCursor(x, y) => {
                self.grid.set_pos(x as usize, y as usize);
            }
            ClientCommand::MoveCursorAbsoluteHorizontal(y) => {
                self.grid.set_pos(self.grid.cursor_pos.0, y as usize);
            }
            ClientCommand::MoveCursorHorizontal(y) => {
                let new_y = self.grid.cursor_pos.1 as i16 + y;
                self.grid.set_pos(self.grid.cursor_pos.0, new_y as usize);
            }
            ClientCommand::MoveCursorVertical(x) => {
                let new_x = self.grid.cursor_pos.0 as i16 + x;
                self.grid.set_pos(new_x as usize, self.grid.cursor_pos.1);
            }
            ClientCommand::ClearLineAfterCursor => {
                let (row, col) = self.grid.cursor_pos;
                self.clear_cells(row, col..self.grid.width as usize);
            }
            ClientCommand::ClearLineBeforeCursor => {
                let (row, col) = self.grid.cursor_pos;
                self.clear_cells(row, 0..col);
            }
            ClientCommand::ClearLine => {
                let (row, _) = self.grid.cursor_pos;
                self.clear_cells(row, 0..self.grid.width as usize);
            }
            ClientCommand::ClearBelow => {
                // first clear after cursor
                let (row, col) = self.grid.cursor_pos;
                self.clear_cells(row, col..self.grid.width as usize);

                // then clear below
                for i in row + 1..self.grid.height as usize {
                    self.clear_cells(i, 0..self.grid.width as usize);
                }
            }
            ClientCommand::ClearAbove => {
                // first clear before cursor
                let (row, col) = self.grid.cursor_pos;
                self.clear_cells(row, 0..col);

                // then clear above
                for i in 0..row {
                    self.clear_cells(i, 0..self.grid.width as usize);
                }
            }
            ClientCommand::ClearCount(count) => {
                let (row, col) = self.grid.cursor_pos;
                self.clear_cells(row, col..col + count as usize);
            }
            ClientCommand::SGR(command) => {
                self.handle_sgr_attribute(command);
            }
            ClientCommand::ReportCursorPosition => self.send_raw_data(
                format!(
                    "\x1b[{};{}R",
                    self.grid.cursor_pos.0 + 1,
                    self.grid.cursor_pos.1 + 1
                )
                .as_bytes()
                .to_vec(),
            ),
            ClientCommand::ReportCondition(healthy) => {
                if healthy {
                    self.send_raw_data(b"\x1b[0n".to_vec());
                } else {
                    self.send_raw_data(b"\x1b[3n".to_vec());
                }
            }
            ClientCommand::ShowCursor => {
                self.grid.show_cursor();
            }
            ClientCommand::PutTab => {
                let (row, col) = self.grid.cursor_pos;
                let grid_len = self.grid.active_grid().len();
                let width = self.grid.width as usize;
                if col < width.saturating_sub(5) {
                    let (fg, bg) = if self.grid.styles.reverse {
                        (
                            self.grid.styles.active_background_color,
                            self.grid.styles.active_text_color,
                        )
                    } else {
                        (
                            self.grid.styles.active_text_color,
                            self.grid.styles.active_background_color,
                        )
                    };
                    for i in col..col + 4 {
                        let index = row * width + i;
                        if index < grid_len {
                            self.grid.active_grid()[index] = Cell::new(' ', fg, bg);
                            self.grid.set_pos(row, i + 1);
                        }
                    }
                }
            }
            ClientCommand::SaveCursor => {
                self.grid.save_cursor();
            }
            ClientCommand::RestoreCursor => {
                self.grid.restore_cursor();
            }
            ClientCommand::SwapScreenAndSetRestoreCursor => {
                self.grid.saved_cursor_pos = self.grid.cursor_pos;
                self.grid.swap_active_grid();
            }
            ClientCommand::IdentifyTerminal(mode) => match mode {
                IdentifyTerminalMode::Primary => {
                    self.send_raw_data(b"\x1b[?6c".to_vec());
                }
                IdentifyTerminalMode::Secondary => {
                    let version = "0.0.1";
                    let text = format!("\x1b[>0;{version};1c");
                    self.send_raw_data(text.as_bytes().to_vec());
                }
            },
            ClientCommand::SetColor(index, color) => {
                self.grid.styles.color_array[index] = Color::Rgb(color.r, color.g, color.b);
            }
            ClientCommand::ResetColor(index) => {
                self.grid.styles.color_array[index] = Color::DEFAULT_ARRAY[index];
            }
            ClientCommand::MoveCursorVerticalWithCarriageReturn(x) => {
                let new_x = self.grid.cursor_pos.0 as i16 + x;
                self.grid.set_pos(new_x as usize, 0);
            }
            ClientCommand::HideCursor => {
                self.grid.hide_cursor();
            }
            ClientCommand::DeleteLines(count) => {
                let (row, _) = self.grid.cursor_pos;
                let width = self.grid.width as usize;
                let height = self.grid.height as usize;
                let count = count as usize;

                // Bounds check - row must be within height
                if row < height {
                    let (fg, bg) = if self.grid.styles.reverse {
                        (
                            self.grid.styles.active_background_color,
                            self.grid.styles.active_text_color,
                        )
                    } else {
                        (
                            self.grid.styles.active_text_color,
                            self.grid.styles.active_background_color,
                        )
                    };

                    // Delete lines at cursor position by shifting lines up
                    let start_idx = row * width;
                    let lines_to_delete = std::cmp::min(count, height.saturating_sub(row));

                    // Remove the lines
                    let remove_count = lines_to_delete * width;
                    let grid = self.grid.active_grid();
                    if start_idx + remove_count <= grid.len() {
                        grid.drain(start_idx..start_idx + remove_count);
                    }

                    // Add blank lines at the bottom to maintain grid size
                    for _ in 0..lines_to_delete {
                        for _ in 0..width {
                            self.grid.active_grid().push(Cell::new(' ', fg, bg));
                        }
                    }
                }
            }
            ClientCommand::SetCursorState(state) => {
                self.grid.styles.cursor_state = state;
            }
            ClientCommand::SetCursorShape(shape) => {
                self.grid.styles.cursor_state.shape = shape;
            }
            ClientCommand::CursorKeysMode(enabled) => {
                self.cursor_keys_mode = enabled;
            }
            ClientCommand::BracketedPasteMode(enabled) => {
                self.bracketed_paste_mode = enabled;
            }
            _ => {
                log::info!("Unsupported command: {:?}", command);
            }
        }
    }

    fn clear_cells(&mut self, row: usize, col_range: std::ops::Range<usize>) {
        let grid_len = self.grid.active_grid().len();
        let width = self.grid.width as usize;

        let start_index = row * width + col_range.start;
        let end_index = row * width + col_range.end;

        // Bounds check to prevent panics after resize
        if start_index >= grid_len {
            return;
        }
        let end_index = std::cmp::min(end_index, grid_len);

        let (fg, bg) = if self.grid.styles.reverse {
            (
                self.grid.styles.active_background_color,
                self.grid.styles.active_text_color,
            )
        } else {
            (
                self.grid.styles.active_text_color,
                self.grid.styles.active_background_color,
            )
        };

        for i in start_index..end_index {
            self.grid.active_grid()[i] = Cell::new(' ', fg, bg);
        }
    }

    fn handle_keyboard_input(&mut self, event: &KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        // Handle special keys
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                self.send_raw_data(vec![8]);
                return;
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.grid.pretty_print();
                self.send_raw_data(vec![27]);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowUp) => {
                // Application mode: ESC O A, Normal mode: ESC [ A
                let seq = if self.cursor_keys_mode { vec![27, 79, 65] } else { vec![27, 91, 65] };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowDown) => {
                let seq = if self.cursor_keys_mode { vec![27, 79, 66] } else { vec![27, 91, 66] };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                let seq = if self.cursor_keys_mode { vec![27, 79, 68] } else { vec![27, 91, 68] };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                let seq = if self.cursor_keys_mode { vec![27, 79, 67] } else { vec![27, 91, 67] };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::Enter) => {
                self.send_raw_data(vec![13]);
                return;
            }
            PhysicalKey::Code(KeyCode::Tab) => {
                self.send_raw_data(vec![9]);
                return;
            }
            PhysicalKey::Code(KeyCode::Space) => {
                self.send_raw_data(vec![32]);
                return;
            }
            _ => {}
        }

        // Handle Ctrl+Shift+I to toggle debug overlay
        if self.modifiers.control_key() && self.modifiers.shift_key() {
            if let PhysicalKey::Code(KeyCode::KeyI) = event.physical_key {
                self.debug_info.show = !self.debug_info.show;
                // Request redraw to show/hide the overlay
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return;
            }
        }

        // Handle Ctrl+key combinations using physical key codes
        if self.modifiers.control_key() {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyC) => {
                    self.send_raw_data(vec![3]);
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyD) => {
                    self.send_raw_data(vec![4]);
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyL) => {
                    self.send_raw_data(vec![12]);
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyU) => {
                    self.send_raw_data(vec![21]);
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyW) => {
                    self.send_raw_data(vec![23]);
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyZ) => {
                    self.send_raw_data(vec![26]);
                    return;
                }
                _ => {}
            }
        }

        // Handle regular text input
        if !self.modifiers.control_key() {
            if let Key::Character(ref text) = event.logical_key {
                self.input.push_str(text);
            }
        }
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        // Immediately resize the renderer for visual feedback
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(new_size);

            let new_width = new_size.width as f32;
            let new_height = new_size.height as f32;

            // Immediately resize grid to match renderer (prevents visual artifacts)
            let (cell_width, cell_height) = renderer.cell_dimensions();
            let new_cols = (new_width / cell_width).floor() as u16;
            let new_rows = (new_height / cell_height).floor() as u16;

            if new_cols != self.grid.width || new_rows != self.grid.height {
                self.grid.resize(new_cols, new_rows);
                self.config.cols = new_cols;
                self.config.rows = new_rows;
                self.config.width = new_width;
                self.config.height = new_height;
            }
        }

        // Debounce only the expensive PTY resize ioctl
        self.pending_resize = Some(new_size);
        self.resize_deadline = Some(Instant::now() + Duration::from_millis(RESIZE_DEBOUNCE_MS));
    }

    fn apply_pending_resize(&mut self) {
        let Some(new_size) = self.pending_resize.take() else {
            return;
        };
        self.resize_deadline = None;

        // Grid and config were already updated in handle_resize
        // Now send the debounced PTY resize command
        log::info!(
            "Sending PTY resize: {} cols, {} rows",
            self.config.cols,
            self.config.rows
        );

        if let Err(e) = self.tx.send(ServerCommand::Resize(
            self.config.cols,
            self.config.rows,
            new_size.width as u16,
            new_size.height as u16,
        )) {
            log::warn!("Failed to send resize command: {}", e);
        }
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
        };

        if y > 0.0 {
            self.grid.scroll_pos = max(
                self.grid.height as usize - 1,
                self.grid.scroll_pos.saturating_sub(1),
            );
        } else {
            self.grid.scroll_pos = min(
                self.grid.active_grid().len().saturating_sub(1),
                self.grid.scroll_pos + 1,
            );
        }
    }

    fn process_commands(&mut self) {
        // Process commands for a limited time to avoid blocking the UI
        let now = std::time::Instant::now();
        while now.elapsed().as_millis() < 50 {
            match self.rx.try_recv() {
                Ok(command) => {
                    self.handle_command(command);
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    break; // No more commands to process
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    log::warn!("UI receiver lagged, {} messages dropped", n);
                    // Continue processing - don't break
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    log::info!("Command channel closed");
                    break;
                }
            }
        }
    }

    fn process_input(&mut self) {
        while !self.input.is_empty() {
            let c = self.input.remove(0);
            self.send_raw_data(vec![c as u8]);
        }
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("MTTY")
                .with_inner_size(PhysicalSize::new(
                    self.config.width as u32,
                    self.config.height as u32,
                ));

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            let renderer = Renderer::new(window.clone(), &self.config);

            // Get actual cell dimensions from renderer and recalculate grid size
            let (cell_width, cell_height) = renderer.cell_dimensions();
            let new_cols = (self.config.width / cell_width).floor() as u16;
            let new_rows = (self.config.height / cell_height).floor() as u16;

            if new_cols != self.config.cols || new_rows != self.config.rows {
                log::info!(
                    "Updating grid size from {}x{} to {}x{} based on actual cell dimensions",
                    self.config.cols,
                    self.config.rows,
                    new_cols,
                    new_rows
                );
                self.config.cols = new_cols;
                self.config.rows = new_rows;
                self.grid = Grid::new(&self.config);

                // Notify the PTY of the correct size
                if let Err(e) = self.tx.send(ServerCommand::Resize(
                    new_cols,
                    new_rows,
                    self.config.width as u16,
                    self.config.height as u16,
                )) {
                    log::warn!("Failed to send resize command: {}", e);
                }
            }

            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.exit_flag
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resize(new_size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(&event);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_wheel(delta);
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render(&mut self.grid, &self.debug_info) {
                        Ok(_) => {
                            self.debug_info.update();
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            renderer.resize(renderer.size());
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("Out of memory");
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::error!("Render error: {:?}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Check if we should exit (e.g., shell process died)
        if self.exit_flag.load(std::sync::atomic::Ordering::Relaxed) {
            event_loop.exit();
            return;
        }

        // Process incoming commands
        self.process_commands();

        // Process buffered input
        self.process_input();

        // Apply debounced resize if deadline has passed
        if let Some(deadline) = self.resize_deadline {
            if Instant::now() >= deadline {
                self.apply_pending_resize();
            }
        }

        // Request redraw when content has changed or debug overlay is shown (for FPS updates)
        if self.grid.is_dirty() || self.debug_info.show {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Always use WaitUntil to avoid busy-looping - never use Poll
        // 8ms gives ~120fps max which is responsive enough for typing
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(8),
        ));
    }
}
