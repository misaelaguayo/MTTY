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
    recording::{Player, Recorder},
    renderer::Renderer,
    snapshot,
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
    pub player: Option<Player>,
    pub auto_record: bool,
}

impl WgpuRunner {
    pub fn new(
        exit_flag: Arc<AtomicBool>,
        config: Config,
        tx: Sender<ServerCommand>,
        rx: Receiver<ClientCommand>,
        player: Option<Player>,
        auto_record: bool,
    ) -> Self {
        Self {
            exit_flag,
            config,
            tx,
            rx,
            player,
            auto_record,
        }
    }
}

impl Runner for WgpuRunner {
    fn run(self) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        // Use Wait instead of Poll to reduce CPU usage when idle
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = WgpuApp::new(
            "MTTY",
            &self.config,
            self.exit_flag.clone(),
            self.tx.clone(),
            self.rx.resubscribe(),
            self.player,
            self.auto_record,
        );

        event_loop.run_app(&mut app).expect("Event loop failed");
    }
}

pub struct WgpuApp {
    title: String,
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
    /// Active recording session (if recording)
    recorder: Option<Recorder>,
    /// Replay player (if in replay mode)
    player: Option<Player>,
    /// Whether replay is currently playing automatically
    replay_playing: bool,
    /// Replay speed: 1 = 1 command, 2 = 10 commands, 3 = 100 commands, etc.
    replay_speed: usize,
    /// Last command executed during replay
    last_replay_command: Option<ClientCommand>,
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title(&self.title)
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

                // In replay mode, don't recreate the grid (it's restored from snapshot)
                if self.player.is_none() {
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

        // Handle replay mode
        if self.player.is_some() {
            if self.replay_playing {
                // Auto-advance in replay mode
                self.replay_step_forward();
                if let Some(ref player) = self.player {
                    if player.is_finished() {
                        self.replay_playing = false;
                        self.update_replay_title();
                    }
                }
            }
        } else {
            // Normal mode: Process incoming commands from PTY
            self.process_commands();

            // Process buffered input
            self.process_input();

            // Apply debounced resize if deadline has passed
            if let Some(deadline) = self.resize_deadline {
                if Instant::now() >= deadline {
                    self.apply_pending_resize();
                }
            }
        }

        // Request redraw when content has changed or debug overlay is shown (for FPS updates)
        if self.grid.is_dirty() || self.debug_info.show {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Control frame rate
        // In replay mode with playing, use faster rate for smoother playback
        let delay = if self.replay_playing { 16 } else { 8 };
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(delay),
        ));
    }
}

impl WgpuApp {
    pub fn new(
        title: &str,
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<ServerCommand>,
        rx: Receiver<ClientCommand>,
        player: Option<Player>,
        auto_record: bool,
    ) -> Self {
        log::info!("Grid size: {} x {}", config.rows, config.cols);

        // If we have a player, initialize grid from the recording's initial state
        let (grid, title) = if let Some(ref p) = player {
            let initial = p.initial_state();
            let mut grid = Grid::new(config);
            grid.restore_from_snapshot(initial);
            let title = format!("MTTY - Replay (0/{})", p.total_events());
            (grid, title)
        } else if auto_record {
            (Grid::new(config), "MTTY - Recording".to_string())
        } else {
            (Grid::new(config), title.to_string())
        };

        // Initialize recorder if auto_record is enabled (and not in replay mode)
        let recorder = if auto_record && player.is_none() {
            log::info!("Auto-recording started");
            Some(Recorder::new(&grid))
        } else {
            None
        };

        Self {
            title,
            exit_flag,
            input: String::new(),
            tx,
            rx,
            config: config.clone(),
            grid,
            window: None,
            renderer: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            pending_resize: None,
            resize_deadline: None,
            debug_info: DebugInfo::new(),
            cursor_keys_mode: false,
            bracketed_paste_mode: false,
            recorder,
            player,
            replay_playing: false,
            replay_speed: 1,
            last_replay_command: None,
        }
    }

    fn send_raw_data(&self, data: Vec<u8>) {
        // Don't send data in replay mode (no PTY)
        if self.player.is_some() {
            return;
        }
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
            SgrAttribute::Foreground(color) => {
                log::debug!("SGR Foreground: {:?}", color);
                self.grid.styles.active_text_color = color;
            }
            SgrAttribute::Background(color) => {
                self.grid.styles.active_background_color = color;
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, command: ClientCommand) {
        let cols = self.grid.width;
        match command {
            ClientCommand::Backspace => {
                self.grid.delete_character();
            }
            ClientCommand::CarriageReturn => {
                self.grid.place_character_in_grid(cols, '\r');
            }
            ClientCommand::ClearScreen => {
                self.grid.clear_screen();
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
            ClientCommand::DeleteChars(count) => {
                self.grid.delete_chars(count as usize);
            }
            ClientCommand::DeleteLines(count) => {
                self.grid.delete_lines(count as usize);
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
            ClientCommand::LineFeed => {
                self.grid.set_pos(self.grid.cursor_pos.0 + 1, 0);
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
            ClientCommand::NewLine => {
                self.grid.place_character_in_grid(cols, '\n');
            }
            ClientCommand::Print(c) => {
                self.grid.place_character_in_grid(cols, c);
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
            ClientCommand::ReportCursorPosition => self.send_raw_data(
                format!(
                    "\x1b[{};{}R",
                    self.grid.cursor_pos.0 + 1,
                    self.grid.cursor_pos.1 + 1
                )
                .as_bytes()
                .to_vec(),
            ),
            ClientCommand::ResetColor(index) => {
                self.grid.styles.color_array[index] = Color::DEFAULT_ARRAY[index];
            }
            ClientCommand::RestoreCursor => {
                self.grid.restore_cursor();
            }
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
            ClientCommand::SGR(command) => {
                self.handle_sgr_attribute(command);
            }
            ClientCommand::SaveCursor => {
                self.grid.save_cursor();
            }
            ClientCommand::SetTitle(title) => {
                if let Some(title_str) = title {
                    self.title = title_str.clone();
                }

                if let Some(window) = &self.window {
                    window.set_title(&self.title);
                }
            }
            ClientCommand::SwapScreenAndSetRestoreCursor(enter) => {
                if enter {
                    // Entering alternate screen: save cursor, switch, clear
                    self.grid.saved_cursor_pos = self.grid.cursor_pos;
                    self.grid.swap_active_grid();
                    self.grid.clear_screen();
                    self.grid.set_pos(0, 0);
                } else {
                    // Exiting alternate screen: switch back, restore cursor
                    self.grid.swap_active_grid();
                    self.grid.cursor_pos = self.grid.saved_cursor_pos;
                    self.grid.mark_all_dirty();
                }
            }
            ClientCommand::SetColor(index, color) => {
                self.grid.styles.color_array[index] = Color::Rgb(color.r, color.g, color.b);
            }
            ClientCommand::MoveCursorVerticalWithCarriageReturn(x) => {
                let new_x = self.grid.cursor_pos.0 as i16 + x;
                self.grid.set_pos(new_x as usize, 0);
            }
            ClientCommand::HideCursor => {
                self.grid.hide_cursor();
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
            ClientCommand::ScrollUp(count) => {
                self.grid.scroll_up(count as usize);
            }
            ClientCommand::ScrollDown(count) => {
                self.grid.scroll_down(count as usize);
            }
            ClientCommand::InsertBlankLines(count) => {
                self.grid.insert_blank_lines(count as usize);
            }
            ClientCommand::SetScrollingRegion(top, bottom) => {
                self.grid.set_scroll_region(top, bottom);
            }
            ClientCommand::ReverseIndex => {
                self.grid.reverse_index();
            }
            ClientCommand::InsertBlanks(count) => {
                self.grid.insert_blanks(count as usize);
            }
            ClientCommand::SetDefaultForeground(rgb) => {
                self.grid.styles.default_text_color = Color::Rgb(rgb.r, rgb.g, rgb.b);
                // Also update active if it's currently using the default
                if matches!(self.grid.styles.active_text_color, Color::Foreground) {
                    self.grid.styles.active_text_color = Color::Foreground;
                }
                self.grid.mark_all_dirty();
            }
            ClientCommand::SetDefaultBackground(rgb) => {
                self.grid.styles.default_background_color = Color::Rgb(rgb.r, rgb.g, rgb.b);
                // Also update active if it's currently using the default
                if matches!(self.grid.styles.active_background_color, Color::Background) {
                    self.grid.styles.active_background_color = Color::Background;
                }
                self.grid.mark_all_dirty();
            }
            ClientCommand::ReportTextAreaSizeChars => {
                // CSI 8 ; rows ; cols t - Report text area size in characters
                let response = format!("\x1b[8;{};{}t", self.grid.height, self.grid.width);
                self.send_raw_data(response.as_bytes().to_vec());
            }
            ClientCommand::ReportTextAreaSizePixels => {
                // CSI 4 ; height ; width t - Report text area size in pixels
                if let Some(renderer) = &self.renderer {
                    let size = renderer.size();
                    let response = format!("\x1b[4;{};{}t", size.height, size.width);
                    self.send_raw_data(response.as_bytes().to_vec());
                }
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

        // Handle replay mode controls FIRST (before normal key handling)
        if self.player.is_some() {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Space) => {
                    self.replay_playing = !self.replay_playing;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyN) => {
                    self.replay_step_forward();
                    return;
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyP) => {
                    self.replay_step_backward();
                    return;
                }
                PhysicalKey::Code(KeyCode::Home) => {
                    self.replay_reset();
                    return;
                }
                PhysicalKey::Code(KeyCode::Escape) => {
                    // Exit replay mode
                    self.exit_flag
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    return;
                }
                // Toggle debug overlay even in replay mode
                PhysicalKey::Code(KeyCode::KeyI)
                    if self.modifiers.control_key() && self.modifiers.shift_key() =>
                {
                    self.debug_info.show = !self.debug_info.show;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                // Replay speed controls: 1-9
                PhysicalKey::Code(KeyCode::Digit1) => {
                    self.replay_speed = 1;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit2) => {
                    self.replay_speed = 2;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit3) => {
                    self.replay_speed = 3;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit4) => {
                    self.replay_speed = 4;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit5) => {
                    self.replay_speed = 5;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit6) => {
                    self.replay_speed = 6;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit7) => {
                    self.replay_speed = 7;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit8) => {
                    self.replay_speed = 8;
                    self.update_replay_title();
                    return;
                }
                PhysicalKey::Code(KeyCode::Digit9) => {
                    self.replay_speed = 9;
                    self.update_replay_title();
                    return;
                }
                _ => {}
            }
            // In replay mode, ignore other keyboard input
            return;
        }

        // Handle special keys (normal mode only)
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                // Send DEL (127) for xterm-256color compatibility, not Ctrl+H (8)
                self.send_raw_data(vec![127]);
                return;
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.send_raw_data(vec![27]);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowUp) => {
                // Application mode: ESC O A, Normal mode: ESC [ A
                let seq = if self.cursor_keys_mode {
                    vec![27, 79, 65]
                } else {
                    vec![27, 91, 65]
                };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowDown) => {
                let seq = if self.cursor_keys_mode {
                    vec![27, 79, 66]
                } else {
                    vec![27, 91, 66]
                };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                let seq = if self.cursor_keys_mode {
                    vec![27, 79, 68]
                } else {
                    vec![27, 91, 68]
                };
                self.send_raw_data(seq);
                return;
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                let seq = if self.cursor_keys_mode {
                    vec![27, 79, 67]
                } else {
                    vec![27, 91, 67]
                };
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

        // Handle Ctrl+Shift shortcuts
        if self.modifiers.control_key() && self.modifiers.shift_key() {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyI) => {
                    // Toggle debug overlay
                    self.debug_info.show = !self.debug_info.show;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyS) => {
                    // Take snapshot
                    self.take_snapshot();
                    return;
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    // Toggle recording (only in normal mode, not replay)
                    if self.player.is_none() {
                        self.toggle_recording();
                    }
                    return;
                }
                _ => {}
            }
        }

        // Handle Ctrl+key combinations using physical key codes
        // Ctrl+A=1, Ctrl+B=2, ..., Ctrl+Z=26
        if self.modifiers.control_key() {
            let ctrl_code = match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyA) => Some(1),
                PhysicalKey::Code(KeyCode::KeyB) => Some(2),
                PhysicalKey::Code(KeyCode::KeyC) => Some(3),
                PhysicalKey::Code(KeyCode::KeyD) => Some(4),
                PhysicalKey::Code(KeyCode::KeyE) => Some(5),
                PhysicalKey::Code(KeyCode::KeyF) => Some(6),
                PhysicalKey::Code(KeyCode::KeyG) => Some(7),
                PhysicalKey::Code(KeyCode::KeyH) => Some(8),
                PhysicalKey::Code(KeyCode::KeyI) => Some(9),
                PhysicalKey::Code(KeyCode::KeyJ) => Some(10),
                PhysicalKey::Code(KeyCode::KeyK) => Some(11),
                PhysicalKey::Code(KeyCode::KeyL) => Some(12),
                PhysicalKey::Code(KeyCode::KeyM) => Some(13),
                PhysicalKey::Code(KeyCode::KeyN) => Some(14),
                PhysicalKey::Code(KeyCode::KeyO) => Some(15),
                PhysicalKey::Code(KeyCode::KeyP) => Some(16),
                PhysicalKey::Code(KeyCode::KeyQ) => Some(17),
                PhysicalKey::Code(KeyCode::KeyR) => Some(18),
                PhysicalKey::Code(KeyCode::KeyS) => Some(19),
                PhysicalKey::Code(KeyCode::KeyT) => Some(20),
                PhysicalKey::Code(KeyCode::KeyU) => Some(21),
                PhysicalKey::Code(KeyCode::KeyV) => Some(22),
                PhysicalKey::Code(KeyCode::KeyW) => Some(23),
                PhysicalKey::Code(KeyCode::KeyX) => Some(24),
                PhysicalKey::Code(KeyCode::KeyY) => Some(25),
                PhysicalKey::Code(KeyCode::KeyZ) => Some(26),
                _ => None,
            };
            if let Some(code) = ctrl_code {
                self.send_raw_data(vec![code]);
                return;
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

        // Don't send resize to PTY in replay mode
        if self.player.is_some() {
            return;
        }

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
                    // Record command if recording is active
                    if let Some(ref mut recorder) = self.recorder {
                        recorder.record_command(&command);
                    }
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

    fn take_snapshot(&mut self) {
        match snapshot::take_snapshot(&self.grid) {
            Ok(path) => {
                log::info!("Snapshot saved to: {:?}", path);
            }
            Err(e) => {
                log::error!("Failed to save snapshot: {}", e);
            }
        }
    }

    fn toggle_recording(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            // Stop recording
            match recorder.finish(&self.grid) {
                Ok(path) => {
                    log::info!("Recording saved to: {:?}", path);
                    self.title = "MTTY".to_string();
                }
                Err(e) => {
                    log::error!("Failed to save recording: {}", e);
                }
            }
        } else {
            // Start recording
            self.recorder = Some(Recorder::new(&self.grid));
            self.title = "MTTY - Recording".to_string();
            log::info!("Recording started");
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title);
        }
    }

    fn replay_step_forward(&mut self) {
        // Calculate number of commands to process based on speed
        // 1 = 1, 2 = 10, 3 = 100, 4 = 1000, etc.
        let commands_to_process = if self.replay_speed == 1 {
            1
        } else {
            10_usize.pow(self.replay_speed as u32 - 1)
        };

        // Collect commands first to avoid borrow issues
        let commands: Vec<ClientCommand> = if let Some(ref mut player) = self.player {
            let mut cmds = Vec::new();
            for _ in 0..commands_to_process {
                if let Some(command) = player.step_forward() {
                    cmds.push(command.clone());
                } else {
                    break;
                }
            }
            cmds
        } else {
            Vec::new()
        };

        // Now process the commands
        for cmd in commands {
            self.last_replay_command = Some(cmd.clone());
            self.handle_command(cmd);
        }

        if self.player.is_some() {
            self.update_replay_title();
        }
    }

    fn replay_step_backward(&mut self) {
        // Calculate number of commands to go back based on speed
        let commands_to_go_back = if self.replay_speed == 1 {
            1
        } else {
            10_usize.pow(self.replay_speed as u32 - 1)
        };

        // First, collect the data we need from the player
        let replay_data = if let Some(ref mut player) = self.player {
            let current_pos = player.position();
            // Calculate target position, ensuring we don't go below 0
            let target_pos = current_pos.saturating_sub(commands_to_go_back);

            if target_pos < current_pos {
                let initial = player.initial_state().clone();
                player.reset();

                // Collect commands up to target position
                let mut commands = Vec::new();
                for _ in 0..target_pos {
                    if let Some(command) = player.step_forward() {
                        commands.push(command.clone());
                    }
                }
                // Seek player to target position
                player.seek(target_pos);
                Some((initial, commands))
            } else {
                None
            }
        } else {
            None
        };

        // Now replay with full ownership of self
        if let Some((initial, commands)) = replay_data {
            self.grid.restore_from_snapshot(&initial);
            // Clear last command if going back to start
            if commands.is_empty() {
                self.last_replay_command = None;
            } else {
                // Track the last command as we replay
                for cmd in commands {
                    self.last_replay_command = Some(cmd.clone());
                    self.handle_command(cmd);
                }
            }
            self.update_replay_title();
        }
    }

    fn replay_reset(&mut self) {
        if let Some(ref mut player) = self.player {
            let initial = player.initial_state().clone();
            self.grid.restore_from_snapshot(&initial);
            player.reset();
            self.replay_playing = false;
            self.last_replay_command = None;
            self.update_replay_title();
        }
    }

    fn update_replay_title(&mut self) {
        if let Some(ref player) = self.player {
            let status = if self.replay_playing {
                "Playing"
            } else {
                "Paused"
            };
            let speed_str = if self.replay_speed == 1 {
                "1".to_string()
            } else {
                format!("10^{}", self.replay_speed - 1)
            };
            let last_cmd = match &self.last_replay_command {
                Some(cmd) => format!("{:?}", cmd),
                None => "None".to_string(),
            };
            // Truncate command display if too long
            let last_cmd_display = if last_cmd.len() > 50 {
                format!("{}...", &last_cmd[..47])
            } else {
                last_cmd
            };
            self.title = format!(
                "MTTY - Replay [{}/{}] {} (x{}) | {}",
                player.position(),
                player.total_events(),
                status,
                speed_str,
                last_cmd_display
            );
            if let Some(window) = &self.window {
                window.set_title(&self.title);
            }
        }
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
