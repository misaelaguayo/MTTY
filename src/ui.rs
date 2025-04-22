use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui::{self, Color32};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{
    commands::{Command, IdentifyTerminalMode, SgrAttribute},
    config::Config,
    grid::Grid,
    styles::{Color, Styles},
};

#[cfg(test)]
mod tests;

pub struct Ui {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
    grid: Grid,
}

impl Ui {
    pub fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<Vec<u8>>,
        rx: Receiver<Command>,
    ) -> Self {
        println!("Grid size: {} x {}", config.rows, config.cols);
        Self {
            exit_flag,
            input: String::new(),
            tx,
            rx,
            grid: Grid::new(config),
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

    fn handle_command(&mut self, command: Command) {
        let cols = self.grid.width;
        match command {
            Command::Backspace => {
                self.grid.delete_character();
            }
            Command::Print(c) => {
                self.grid.place_character_in_grid(cols, c);
            }
            Command::NewLine => {
                self.grid.place_character_in_grid(cols, '\n');
            }
            Command::CarriageReturn => {
                self.grid.place_character_in_grid(cols, '\r');
            }
            Command::LineFeed => {
                self.grid.set_pos(self.grid.cursor_pos.0 + 1, 0);
            }
            Command::ClearScreen => {
                self.grid.clear_screen();
            }
            Command::MoveCursor(x, y) => {
                self.grid.set_pos(x as usize, y as usize);
            }
            Command::MoveCursorAbsoluteHorizontal(y) => {
                self.grid.set_pos(self.grid.cursor_pos.0, y as usize);
            }
            Command::MoveCursorHorizontal(y) => {
                let new_y = self.grid.cursor_pos.1 as i16 + y;
                self.grid.set_pos(self.grid.cursor_pos.0, new_y as usize);
            }
            Command::MoveCursorVertical(x) => {
                let new_x = self.grid.cursor_pos.1 as i16 + x;
                self.grid.set_pos(new_x as usize, self.grid.cursor_pos.0);
            }
            Command::ClearLineAfterCursor => {
                let (row, col) = self.grid.cursor_pos;
                for i in col..self.grid.width as usize {
                    self.grid.active_grid()[row][i].char = ' ';
                }
            }
            Command::ClearLineBeforeCursor => {
                let (row, col) = self.grid.cursor_pos;
                for i in 0..col {
                    self.grid.active_grid()[row][i].char = ' ';
                }
            }
            Command::ClearLine => {
                let (row, _) = self.grid.cursor_pos;
                for i in 0..self.grid.width as usize {
                    self.grid.active_grid()[row][i].char = ' ';
                }
            }
            Command::ClearBelow => {
                // first clear after cursor
                let (row, col) = self.grid.cursor_pos;
                for i in col..self.grid.width as usize {
                    self.grid.active_grid()[row][i].char = ' ';
                }

                // then clear below
                for i in row + 1..self.grid.height as usize {
                    for j in 0..self.grid.width as usize {
                        self.grid.active_grid()[i][j].char = ' ';
                    }
                }
            }
            Command::ClearAbove => {
                // first clear before cursor
                let (row, col) = self.grid.cursor_pos;
                for i in 0..col {
                    self.grid.active_grid()[row][i].char = ' ';
                }

                // then clear above
                for i in 0..row {
                    for j in 0..self.grid.width as usize {
                        self.grid.active_grid()[i][j].char = ' ';
                    }
                }
            }
            Command::ClearCount(count) => {
                let (row, col) = self.grid.cursor_pos;
                for i in 0..count {
                    if col + i as usize >= self.grid.width as usize {
                        break;
                    }
                    self.grid.active_grid()[row][col + i as usize].char = ' ';
                }
            }
            Command::SGR(command) => {
                self.handle_sgr_attribute(command);
            }
            Command::ReportCursorPosition => {
                self.tx
                    .send(
                        format!(
                            "\x1b[{};{}R",
                            self.grid.cursor_pos.0, self.grid.cursor_pos.1
                        )
                        .as_bytes()
                        .to_vec(),
                    )
                    .unwrap();
            }
            Command::ReportCondition(healthy) => {
                if healthy {
                    self.tx.send(b"\x1b[0n".to_vec()).unwrap();
                } else {
                    self.tx.send(b"\x1b[3n".to_vec()).unwrap();
                }
            }
            Command::ShowCursor => {
                self.grid.show_cursor();
            }
            Command::PutTab => {
                let (row, col) = self.grid.cursor_pos;
                if col < self.grid.width as usize - 5 {
                    for i in col..col + 4 {
                        self.grid.active_grid()[row][i].char = ' ';
                        self.grid.set_pos(row, i + 1);
                    }
                }
            }
            Command::SaveCursor => {
                self.grid.save_cursor();
            }
            Command::RestoreCursor => {
                self.grid.restore_cursor();
            }
            Command::SwapScreenAndSetRestoreCursor => {
                self.grid.saved_cursor_pos = self.grid.cursor_pos;
                self.grid.swap_active_grid();
            }
            Command::IdentifyTerminal(mode) => match mode {
                IdentifyTerminalMode::Primary => {
                    self.tx.send(b"\x1b[?6c".to_vec()).unwrap();
                }
                IdentifyTerminalMode::Secondary => {
                    let version = "0.0.1";
                    let text = format!("\x1b[>0;{version};1c");
                    self.tx.send(text.as_bytes().to_vec()).unwrap();
                }
            },
            Command::SetColor(index, color) => {
                self.grid.styles.color_array[index] = Color::Rgb(color.r, color.g, color.b);
            }
            Command::ResetColor(index) => {
                self.grid.styles.color_array[index] = Color::default_array()[index];
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, event: &egui::Event) {
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
                        self.tx.send(vec![8]).unwrap();
                    }
                    egui::Key::Escape => {
                        self.grid.pretty_print();
                        self.tx.send(vec![27]).unwrap();
                    }
                    egui::Key::ArrowUp => {
                        self.tx.send(vec![27, 91, 65]).unwrap();
                    }
                    egui::Key::ArrowDown => {
                        self.tx.send(vec![27, 91, 66]).unwrap();
                    }
                    egui::Key::ArrowLeft => {
                        self.tx.send(vec![27, 91, 68]).unwrap();
                    }
                    egui::Key::ArrowRight => {
                        self.tx.send(vec![27, 91, 67]).unwrap();
                    }
                    egui::Key::Enter => {
                        self.tx.send(vec![13]).unwrap();
                    }
                    egui::Key::Tab => {
                        self.tx.send(vec![9]).unwrap();
                    }
                    _ => {}
                }

                match modifiers {
                    egui::Modifiers { ctrl: true, .. } => match key.name() {
                        "C" => {
                            self.tx.send(vec![3]).unwrap();
                        }
                        "D" => {
                            self.tx.send(vec![4]).unwrap();
                        }
                        "L" => {
                            self.tx.send(vec![12]).unwrap();
                        }
                        "U" => {
                            self.tx.send(vec![21]).unwrap();
                        }
                        "W" => {
                            self.tx.send(vec![23]).unwrap();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            egui::Event::Text(text) => {
                self.input.push_str(text);
            }
            _ => {}
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(data) = self.rx.try_recv() {
            self.handle_command(data);
        }

        if !self.input.is_empty() {
            let _ = self.tx.send(self.input.as_bytes().to_vec());

            self.input.clear();
        }

        let frame = egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin::ZERO,
            ..Default::default()
        };

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.input(|i| {
                i.raw.events.iter().for_each(|event| {
                    self.handle_event(event);
                });
            });

            egui::Grid::new("grid")
                .striped(false)
                .min_col_width(0.0)
                .max_col_width(10.0)
                .min_row_height(0.0)
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    let start_row = 0;

                    for i in start_row..self.grid.height as usize {
                        for j in 0..self.grid.width as usize {
                            let cell = self.grid.active_grid()[i][j].clone();
                            let fg = self.grid.styles.to_color32(cell.fg);
                            let bg = if i == self.grid.cursor_pos.0 && j == self.grid.cursor_pos.1 {
                                Color32::WHITE
                            } else {
                                self.grid.styles.to_color32(cell.bg)
                            };

                            ui.monospace(
                                egui::RichText::new(cell.to_string())
                                    .color(fg)
                                    .background_color(bg),
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
