use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui::{self};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{commands::Command, config::Config};

#[cfg(test)]
mod tests;

pub struct Ui {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
    pos: (usize, usize),
    grid: Vec<Vec<char>>,
}

impl Ui {
    pub fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<Vec<u8>>,
        rx: Receiver<Command>,
    ) -> Self {
        Self {
            exit_flag,
            input: String::new(),
            tx,
            rx,
            pos: (0, 0),
            grid: vec![vec![' '; config.rows as usize]; config.cols as usize],
        }
    }

    fn set_pos(&mut self, x: usize, y: usize) {
        self.pos = (x, y);
    }

    fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (x, y) = self.pos;

        match c {
            '\n' => {
                self.set_pos(x, y + 1);
            }
            '\r' => {
                self.set_pos(0, y);
            }
            _ => {
                self.grid[x][y] = c;
                self.set_pos(x + 1, y);
            }
        }

        if x >= cols as usize - 1 {
            self.set_pos(0, y + 1);
        }
    }

    fn clear_screen(&mut self) {
        self.grid = vec![vec![' '; self.grid[0].len()]; self.grid.len()];
    }

    fn delete_character(&mut self) {
        let (mut x, mut y) = self.pos;
        let cols = self.grid[0].len() as usize;

        if x > 0 {
            (x, y) = self.pos;
            self.grid[x][y] = ' ';

            self.set_pos(x - 1, y);
        } else if y > 0 {
            (x, y) = self.pos;
            self.grid[x][y] = ' ';

            self.set_pos(cols - 1, y - 1);
        } else {
            self.grid[x][y] = ' ';
        }
    }

    fn handle_command(&mut self, command: Command) {
        let cols = self.grid.len() as u16;
        match command {
            Command::Backspace => {
                self.delete_character();
            }
            Command::Print(c) => {
                self.place_character_in_grid(cols, c);
            }
            Command::NewLine => {
                self.place_character_in_grid(cols, '\n');
            }
            Command::CarriageReturn => {
                self.place_character_in_grid(cols, '\r');
            }
            Command::ClearScreen => {
                self.clear_screen();
            }
            Command::MoveCursor(x, y) => {
                self.set_pos(x as usize, y as usize);
            }
            Command::MoveCursorAbsoluteHorizontal(x) => {
                self.set_pos(x as usize, self.pos.1);
            }
            Command::MoveCursorHorizontal(x) => {
                let new_x = self.pos.0 as i16 + x;
                self.set_pos(new_x as usize, self.pos.1);
            }
            Command::MoveCursorVertical(y) => {
                let new_y = self.pos.1 as i16 + y;
                self.set_pos(self.pos.0, new_y as usize);
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, event: &egui::Event) {
        match event {
            egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                ..
            } => {
                self.input.push('\n');
            }
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                ..
            } => {
                // ASCII code for backspace
                self.delete_character();
                self.tx.try_send(vec![8]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                self.tx.try_send(vec![27]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Space,
                pressed: true,
                ..
            } => {
                self.input.push(' ');
            }
            egui::Event::Key {
                key: egui::Key::Minus,
                pressed: true,
                ..
            } => {
                self.input.push('-');
            }
            egui::Event::Key {
                key: egui::Key::Period,
                pressed: true,
                ..
            } => {
                self.input.push('.');
            }
            egui::Event::Key {
                key: egui::Key::ArrowUp,
                pressed: true,
                ..
            } => {
                self.tx.try_send(vec![27, 91, 65]).unwrap();
            }
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } => match modifiers {
                egui::Modifiers { shift: true, .. } => {
                    self.input.push_str(&key.name());
                }
                egui::Modifiers { ctrl: true, .. } => match key.name() {
                    "C" => {
                        self.tx.try_send(vec![3]).unwrap();
                    }
                    "D" => {
                        self.tx.try_send(vec![4]).unwrap();
                    }
                    "L" => {
                        self.tx.try_send(vec![12]).unwrap();
                    }
                    "U" => {
                        self.tx.try_send(vec![21]).unwrap();
                    }
                    "W" => {
                        self.tx.try_send(vec![23]).unwrap();
                    }
                    "R" => {
                        // dev command to display current cursor position
                        self.tx.try_send("\\x1b[6n".as_bytes().to_vec()).unwrap();
                    }
                    _ => {}
                },
                _ => {
                    self.input.push_str(&key.name().to_lowercase());
                }
            },
            _ => {}
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TODO: Looks like accessing the row size and column size from the config struct
        // takes a long time. Currently using hardcoded values for the grid size.
        let rows = 35;
        let cols = 106 as u16;

        if let Some(data) = self.rx.try_recv().ok() {
            self.handle_command(data);
        }

        if !self.input.is_empty() {
            let _ = self.tx.try_send(self.input.as_bytes().to_vec());

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
                .min_col_width(0.0001)
                .max_col_width(0.0001)
                .min_row_height(0.0001)
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    for row in 0..rows - 1 {
                        for col in 0..cols - 1 {
                            ui.monospace(self.grid[col as usize][row].to_string());
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
