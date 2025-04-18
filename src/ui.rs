use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui::{self, Color32};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    commands::{Command, IdentifyTerminalMode},
    config::Config,
    styles::Styles,
};

#[cfg(test)]
mod tests;

pub struct Ui {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
    pos: (usize, usize),
    saved_pos: (usize, usize),
    grid: Vec<Vec<char>>,
    cols: usize,
    rows: usize,
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
            saved_pos: (0, 0),
            grid,
            cols: config.cols as usize,
            rows: config.rows as usize,
            styles: Styles::default(),
        }
    }

    #[cfg(debug_assertions)]
    fn pretty_print_grid(&self) {
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

    fn double_grid(&mut self) {
        let cols = self.grid[0].len();
        self.grid
            .resize_with(self.grid.len() * 2, || vec![' '; cols]);
    }

    fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (mut row, mut col) = self.pos;

        if col >= cols as usize - 1 {
            self.set_pos(row + 1, 0);
        }

        (row, col) = self.pos;
        while row >= self.grid.len() {
            self.double_grid();
        }

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

    fn show_cursor(&mut self) {
        let (row, col) = self.pos;
        if row < self.grid.len() && col < self.grid[row].len() {
            self.grid[row][col] = '|';
        }
    }

    fn save_cursor(&mut self) {
        self.saved_pos = self.pos;
    }

    fn restore_cursor(&mut self) {
        self.pos = self.saved_pos;
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
            Command::LineFeed => {
                self.set_pos(self.pos.0 + 1, 0);
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
                    if col + i as usize >= self.grid[row].len() {
                        break;
                    }
                    self.grid[row][col + i as usize] = ' ';
                }
            }
            Command::SGR(commands) => {
                commands.iter().for_each(|command| {
                    self.handle_sgr_command(*command);
                });
            }
            Command::ReportCursorPosition => {
                self.tx
                    .try_send(
                        format!("\x1b[{};{}R", self.pos.0, self.pos.1)
                            .as_bytes()
                            .to_vec(),
                    )
                    .unwrap();
            }
            Command::ReportCondition(healthy) => {
                if healthy {
                    self.tx.try_send(b"\x1b[0n".to_vec()).unwrap();
                } else {
                    self.tx.try_send(b"\x1b[3n".to_vec()).unwrap();
                }
            }
            Command::ShowCursor => {
                self.show_cursor();
            }
            Command::PutTab => {
                let (row, col) = self.pos;
                if col < self.grid[row].len() - 5 {
                    for i in col..col + 4 {
                        self.grid[row][i] = ' ';
                        self.set_pos(row, i + 1);
                    }
                }
            }
            Command::SaveCursor => {
                self.save_cursor();
            }
            Command::RestoreCursor => {
                self.restore_cursor();
            }
            Command::SwapScreenAndSetRestoreCursor => {
                self.saved_pos = self.pos;
                self.grid = vec![vec![' '; self.grid[0].len()]; self.grid.len()];
            }
            Command::IdentifyTerminal(mode) => match mode {
                IdentifyTerminalMode::Primary => {
                    self.tx.try_send(b"\x1b[?6c".to_vec()).unwrap();
                }
                IdentifyTerminalMode::Secondary => {
                    let version = "0.0.1";
                    let text = format!("\x1b[>0;{version};1c");
                    self.tx.try_send(text.as_bytes().to_vec()).unwrap();
                }
            },
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
                        self.tx.try_send(vec![8]).unwrap();
                    }
                    egui::Key::Escape => {
                        self.pretty_print_grid();
                        self.tx.try_send(vec![27]).unwrap();
                    }
                    egui::Key::ArrowUp => {
                        self.tx.try_send(vec![27, 91, 65]).unwrap();
                    }
                    egui::Key::Enter => {
                        self.tx.try_send(vec![13]).unwrap();
                    }
                    egui::Key::Tab => {
                        self.tx.try_send(vec![9]).unwrap();
                    }
                    _ => {}
                }

                match modifiers {
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
                    // Show only the last `rows` rows of the grid
                    let start_row = self.grid.len() - self.rows;

                    for (i, row) in self.grid[start_row..].iter().enumerate() {
                        for (j, c) in row.iter().enumerate() {
                            if i == self.pos.0 && j == self.pos.1 {
                                ui.monospace(
                                    egui::RichText::new(c.to_string())
                                        .color(self.styles.text_color.to_color32())
                                        .background_color(Color32::WHITE),
                                );
                            } else {
                                ui.monospace(
                                    egui::RichText::new(c.to_string())
                                        .color(self.styles.text_color.to_color32()),
                                );
                            }
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
