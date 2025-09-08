use std::{
    cmp::{max, min},
    sync::{atomic::AtomicBool, Arc},
};

use eframe::egui::{self, Pos2};
use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::{
    commands::{ClientCommand, IdentifyTerminalMode, ServerCommand, SgrAttribute},
    config::Config,
    fonts,
    grid::{Cell, Grid},
    styles::{Color, Styles},
};

#[cfg(test)]
mod tests;

// Trait defining a runner that can execute the UI
// This allows for different implementations of the UI e.g. Egui, Iced, etc.
pub trait Runner {
    fn run(self);
}

pub struct EguiRunner {
    pub exit_flag: Arc<AtomicBool>,
    pub config: Config,
    pub tx: Sender<ServerCommand>,
    pub rx: Receiver<ClientCommand>,
}

impl Runner for EguiRunner {
    fn run(self) {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_icon(eframe::egui::IconData::default())
                .with_inner_size([self.config.width, self.config.height]),
            ..Default::default()
        };

        let egui_ui = EguiApp::new(
            &self.config,
            self.exit_flag.clone(),
            self.tx.clone(),
            self.rx.resubscribe(),
        );

        eframe::run_native(
            "MTTY",
            options,
            Box::new(|cc| {
                let ctx = cc.egui_ctx.clone();
                fonts::configure_text_styles(&ctx, &self.config);
                tokio::spawn(async move {
                    redraw(ctx, self.rx, self.exit_flag);
                });

                Ok(Box::new(egui_ui))
            }),
        )
        .unwrap_or_else(|e| {
            log::error!("Failed to start egui UI: {}", e);
        });
    }
}

pub struct EguiApp {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<ServerCommand>,
    rx: Receiver<ClientCommand>,
    config: Config,
    grid: Grid,
}

impl EguiApp {
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
        }
    }

    fn send_raw_data(&self, data: Vec<u8>) {
        self.tx
            .send(ServerCommand::RawData(data))
            .expect("Failed to send raw data");
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
            SgrAttribute::Reverse => {}
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
            SgrAttribute::CancelReverse => {}
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
                    self.grid.cursor_pos.0, self.grid.cursor_pos.1
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
                if col < self.grid.width as usize - 5 {
                    for i in col..col + 4 {
                        self.grid.active_grid()[row][i] = Cell::new(
                            ' ',
                            self.grid.styles.active_text_color,
                            self.grid.styles.active_background_color,
                        );
                        self.grid.set_pos(row, i + 1);
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
                // delete lines at cursor position

                for _ in row..row + count as usize + 1 {
                    self.grid.active_grid().remove(row);
                }
            }
            ClientCommand::SetCursorState(state) => {
                self.grid.styles.cursor_state = state;
            }
            ClientCommand::SetCursorShape(shape) => {
                self.grid.styles.cursor_state.shape = shape;
            }
            _ => {
                log::info!("Unsupported command: {:?}", command);
            }
        }
    }

    fn clear_cells(&mut self, row: usize, col_range: std::ops::Range<usize>) {
        for i in col_range {
            self.grid.active_grid()[row][i] = Cell::new(
                ' ',
                self.grid.styles.active_text_color,
                self.grid.styles.active_background_color,
            );
        }
    }

    fn handle_event(&mut self, event: &egui::Event, viewport: Option<egui::Rect>) {
        if let Some(rect) = viewport {
            let Pos2 { x: x0, y: y0 } = rect.min;
            let Pos2 { x: x1, y: y1 } = rect.max;

            let new_width = (x1 - x0) as usize;
            let new_height = (y1 - y0) as usize;

            if new_width != self.config.width as usize || new_height != self.config.height as usize
            {
                log::info!(
                    "Viewport changed: new width = {}, new height = {}",
                    new_width,
                    new_height
                );

                self.config.width = new_width as f32;
                self.config.height = new_height as f32;

                let (new_cols, new_rows) = self
                    .config
                    .get_col_rows_from_size(self.config.width, self.config.height);

                self.tx
                    .send(ServerCommand::Resize(
                        new_cols,
                        new_rows,
                        new_width as u16,
                        new_height as u16,
                    ))
                    .expect("Failed to send resize command");
            }
        }

        match event {
            egui::Event::Key {
                key,
                modifiers,
                repeat: false,
                pressed: true,
                ..
            } => {
                match key {
                    egui::Key::Backspace => {
                        self.send_raw_data(vec![8]);
                    }
                    egui::Key::Escape => {
                        self.grid.pretty_print();
                        self.send_raw_data(vec![27]);
                    }
                    egui::Key::ArrowUp => {
                        self.send_raw_data(vec![27, 91, 65]);
                    }
                    egui::Key::ArrowDown => {
                        self.send_raw_data(vec![27, 91, 66]);
                    }
                    egui::Key::ArrowLeft => {
                        self.send_raw_data(vec![27, 91, 68]);
                    }
                    egui::Key::ArrowRight => {
                        self.send_raw_data(vec![27, 91, 67]);
                    }
                    egui::Key::Enter => {
                        self.send_raw_data(vec![13]);
                    }
                    egui::Key::Tab => {
                        self.send_raw_data(vec![9]);
                    }
                    _ => {}
                }

                match modifiers {
                    egui::Modifiers { ctrl: true, .. } => match key.name() {
                        "C" => {
                            self.send_raw_data(vec![3]);
                        }
                        "D" => {
                            self.send_raw_data(vec![4]);
                        }
                        "L" => {
                            self.send_raw_data(vec![12]);
                        }
                        "U" => {
                            self.send_raw_data(vec![21]);
                        }
                        "W" => {
                            self.send_raw_data(vec![23]);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            egui::Event::Text(text) => {
                self.input.push_str(text);
            }
            egui::Event::MouseWheel { delta, .. } => {
                let y = delta.y;
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
            _ => {}
        }
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process commands for a limited time to avoid blocking the UI
        let now = std::time::Instant::now();
        while now.elapsed().as_millis() < 50 {
            match self.rx.try_recv() {
                Ok(command) => {
                    self.handle_command(command);
                }
                Err(_) => {
                    break; // No more commands to process
                }
            }
        }

        while self.input.len() > 0 {
            let c = self.input.remove(0);
            self.send_raw_data(vec![c as u8]);
        }

        let frame = egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::ZERO,
            ..Default::default()
        };

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.input(|i| {
                i.raw.events.iter().for_each(|event| {
                    self.handle_event(event, i.viewport().inner_rect);
                });
            });

            egui::Grid::new("grid")
                .striped(false)
                .min_col_width(0.0)
                .max_col_width(10.0)
                .min_row_height(0.0)
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    let start_row = self
                        .grid
                        .scroll_pos
                        .saturating_sub(self.grid.height as usize);
                    let end_row = self.grid.active_grid().len();

                    for i in start_row..end_row as usize {
                        for j in 0..self.grid.width as usize {
                            let cell = self.grid.active_grid()[i][j].clone();

                            let cell_text =
                                if i == self.grid.cursor_pos.0 && j == self.grid.cursor_pos.1 {
                                    self.grid.styles.cursor_state.to_string()
                                } else {
                                    cell.to_string()
                                };

                            ui.monospace(
                                egui::RichText::new(cell_text)
                                    .color(self.grid.styles.to_color32(cell.fg))
                                    .background_color(self.grid.styles.to_color32(cell.bg)),
                            );
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

fn redraw(
    ctx: egui::Context,
    mut rx: broadcast::Receiver<ClientCommand>,
    exit_flag: Arc<AtomicBool>,
) {
    loop {
        if exit_flag.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        while let Ok(_) = rx.try_recv() {
            ctx.request_repaint();
        }
    }
}
