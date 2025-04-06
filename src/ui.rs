use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui::{self};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{commands::Command, config::Config, styles::Styles};

#[cfg(test)]
mod tests;

pub struct Ui {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
    pos: (usize, usize),
    grid: Vec<Vec<char>>,
    styles: Styles,
}

impl Ui {
    pub fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<Vec<u8>>,
        rx: Receiver<Command>,
    ) -> Self {
        let grid = vec![vec![' '; config.cols as usize]; config.rows as usize];
        println!("Grid size: {} x {}", config.rows, config.cols);
        Self {
            exit_flag,
            input: String::new(),
            tx,
            rx,
            pos: (0, 0),
            grid,
            styles: Styles::default(),
        }
    }

    #[cfg(debug_assertions)]
    fn _pretty_print_grid(&self) {
        for row in &self.grid {
            for c in row {
                if *c == ' ' {
                    print!(".");
                } else {
                    print!("{}", c);
                }
            }
            println!();
        }
    }

    fn set_pos(&mut self, x: usize, y: usize) {
        self.pos = (x, y);
    }

    fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (row, col) = self.pos;

        match c {
            '\n' => {
                self.set_pos(row + 1, 0);
            }
            '\r' => {
                self.set_pos(row, 0);
            }
            _ => {
                self.grid[row][col] = c;
                self.set_pos(row, col + 1);
            }
        }

        if col >= cols as usize - 1 {
            self.set_pos(row + 1, 0);
        }
    }

    fn clear_screen(&mut self) {
        self.grid = vec![vec![' '; self.grid[0].len()]; self.grid.len()];
    }

    fn delete_character(&mut self) {
        let (mut row, mut col) = self.pos;
        let cols = self.grid[0].len() as usize;

        if col > 0 {
            (row, col) = self.pos;
            self.grid[row][col] = ' ';

            self.set_pos(row, col - 1);
        } else if row > 0 {
            (row, col) = self.pos;
            self.grid[row][col] = ' ';

            self.set_pos(row - 1, cols - 1);
        } else {
            self.grid[row][col] = ' ';
        }
    }

    fn handle_sgr_command(&mut self, command: i16) {
        match command {
            0 => {
                // reset all styles
                self.styles = Styles::default();
            }
            1 => {
                // bold
                self.styles.font_size = 20;
            }
            2 => {
                // faint
                self.styles.font_size = 14;
            }
            3 => {
                // italic
                self.styles.italic = true;
            }
            4 => {
                // underline
                self.styles.underline = true;
            }
            30..37 => {
                // foreground color
                self.styles.set_foreground_color_from_int(command);
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, command: Command) {
        let cols = self.grid[0].len() as u16;
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
                self.set_pos(y as usize, x as usize);
            }
            Command::MoveCursorAbsoluteHorizontal(y) => {
                self.set_pos(self.pos.0, y as usize);
            }
            Command::MoveCursorHorizontal(y) => {
                let new_y = self.pos.1 as i16 + y;
                self.set_pos(self.pos.0, new_y as usize);
            }
            Command::MoveCursorVertical(x) => {
                let new_x = self.pos.1 as i16 + x;
                self.set_pos(new_x as usize, self.pos.0);
            }
            Command::ClearLineAfterCursor => {
                let (row, col) = self.pos;
                for i in col..self.grid[row].len() {
                    self.grid[row][i] = ' ';
                }
            }
            Command::ClearLineBeforeCursor => {
                let (row, col) = self.pos;
                for i in 0..col {
                    self.grid[row][i] = ' ';
                }
            }
            Command::ClearLine => {
                let (row, _) = self.pos;
                for i in 0..self.grid[row].len() {
                    self.grid[row][i] = ' ';
                }
            }
            Command::ClearBelow => {
                // first clear after cursor
                let (row, col) = self.pos;
                for i in col..self.grid[row].len() {
                    self.grid[row][i] = ' ';
                }

                // then clear below
                for i in row + 1..self.grid.len() {
                    for j in 0..self.grid[i].len() {
                        self.grid[i][j] = ' ';
                    }
                }
            }
            Command::ClearAbove => {
                // first clear before cursor
                let (row, col) = self.pos;
                for i in 0..col {
                    self.grid[row][i] = ' ';
                }

                // then clear above
                for i in 0..row {
                    for j in 0..self.grid[i].len() {
                        self.grid[i][j] = ' ';
                    }
                }
            }
            Command::ClearCount(count) => {
                let (row, col) = self.pos;
                for i in 0..count {
                    // TODO: check if the index is within bounds
                    self.grid[row][col + i as usize] = ' ';
                }
            }
            Command::SGR(commands) => {
                commands.iter().for_each(|command| {
                    self.handle_sgr_command(*command);
                });
            }
            // Command::ReportCursorPosition => {
            //     self.tx
            //         .send(format!("\x1b[{};{}R", self.pos.0 + 1, self.pos.1 + 1).as_bytes().to_vec())
            //         .unwrap();
            // }
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
                self.tx.send(vec![8]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                // self.pretty_print_grid();
                self.tx.send(vec![27]).unwrap();
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
                self.tx.send(vec![27, 91, 65]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Colon,
                pressed: true,
                ..
            } => {
                self.input.push(':');
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
                    "R" => {
                        // dev command to display current cursor position
                        self.tx.send("\\x1b[6n".as_bytes().to_vec()).unwrap();
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
        // let rows = 35;
        // let cols = 106 as u16;

        if let Some(data) = self.rx.try_recv().ok() {
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
                .min_col_width(0.0001)
                .max_col_width(0.0001)
                .min_row_height(0.0001)
                .spacing([0.0, 0.0])
                .show(ui, |ui| {
                    self.grid.iter().for_each(|row| {
                        row.iter().for_each(|&c| {
                            ui.monospace(
                                egui::RichText::new(c.to_string())
                                    .color(self.styles.text_color.to_color32()),
                            );
                        });
                        ui.end_row();
                    });
                });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
