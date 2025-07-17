use crate::{
    commands::SgrAttribute,
    config::Config,
    styles::{Color, Styles},
};
use std::fmt;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Vec<SgrAttribute>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: Color::White,
            bg: Color::Black,
            attrs: vec![SgrAttribute::default()],
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.char)
    }
}

impl Cell {
    pub fn new(c: char, fg: Color, bg: Color) -> Self {
        Self {
            char: c,
            fg,
            bg,
            attrs: vec![],
        }
    }
}

pub struct Grid {
    cells: Vec<Vec<Cell>>,
    alternate_screen: Vec<Vec<Cell>>,
    alternate: bool,
    pub width: u16,
    pub height: u16,
    pub cursor_pos: (usize, usize),
    pub saved_cursor_pos: (usize, usize),
    pub scroll_pos: usize,
    pub styles: Styles,
}

impl Grid {
    pub fn new(config: &Config) -> Self {
        let width = config.cols;
        let height = config.rows;
        let cells = vec![vec![Cell::default(); width as usize]; height as usize];
        let alternate_screen = vec![vec![Cell::default(); width as usize]; height as usize];

        Self {
            width,
            height,
            cells,
            alternate_screen,
            cursor_pos: (0, 0),
            saved_cursor_pos: (0, 0),
            scroll_pos: height as usize - 1,
            styles: Styles::default(),
            alternate: false,
        }
    }

    pub fn active_grid(&mut self) -> &mut Vec<Vec<Cell>> {
        if self.alternate {
            &mut self.alternate_screen
        } else {
            &mut self.cells
        }
    }

    pub fn swap_active_grid(&mut self) {
        self.alternate = !self.alternate;
        if !self.alternate {
            self.alternate_screen = vec![
                vec![
                    Cell::new(
                        ' ',
                        self.styles.active_text_color,
                        self.styles.active_background_color
                    );
                    self.width as usize
                ];
                self.height as usize
            ];
        }
    }

    pub fn pretty_print(&mut self) {
        println!("Grid: {}x{}", self.width, self.height);
        println!("Cursor Position: {:?}", self.cursor_pos);
        println!("Saved Cursor Position: {:?}", self.saved_cursor_pos);
        println!("Scroll Position: {:?}", self.scroll_pos);
        println!(
            "Active Grid: {:?}",
            if self.alternate { "Alternate" } else { "Main" }
        );
        println!(
            "Colors: Bg: {:?}, Fg: {:?}",
            self.styles.active_background_color, self.styles.active_text_color
        );

        for row in self.active_grid() {
            for cell in row {
                if cell.char == ' ' {
                    print!(".");
                } else {
                    print!("{}", cell.char);
                }
            }
            println!();
        }
    }

    pub fn set_pos(&mut self, row: usize, col: usize) {
        let rows = self.active_grid().len();
        if row >= rows {
            self.add_rows(row - rows + 1);
            self.scroll_pos = row;
        }

        self.cursor_pos = (row, col);
    }

    pub fn add_rows(&mut self, rows: usize) {
        let cols = self.width;
        let curr_rows = self.active_grid().len();
        let fg = self.styles.active_text_color;
        let bg = self.styles.active_background_color;

        self.active_grid().resize_with(curr_rows + rows, || {
            vec![Cell::new(' ', fg, bg); cols as usize]
        });
    }

    pub fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (mut row, mut col) = self.cursor_pos;

        if col >= cols as usize - 1 {
            self.set_pos(row + 1, 0);
        }

        (row, col) = self.cursor_pos;
        let fg = self.styles.active_text_color;
        let bg = self.styles.active_background_color;

        match c {
            '\n' => {
                self.set_pos(row + 1, 0);
            }
            '\r' => {
                self.set_pos(row, 0);
            }
            _ => {
                self.active_grid()[row][col] = Cell::new(c, fg, bg);
                self.set_pos(row, col + 1);
            }
        }
    }

    pub fn clear_screen(&mut self) {
        let fg = self.styles.active_text_color;
        let bg = self.styles.active_background_color;

        for row in self.active_grid() {
            for cell in row {
                *cell = Cell::new(' ', fg, bg);
            }
        }

        self.scroll_pos = 0;
    }

    pub fn delete_character(&mut self) {
        let (row, col) = self.cursor_pos;
        let cols = self.width as usize;
        let fg = self.styles.active_text_color;
        let bg = self.styles.active_background_color;

        self.active_grid()[row][col] = Cell::new(' ', fg, bg);

        if col > 0 {
            self.set_pos(row, col - 1);
        } else if row > 0 {
            self.set_pos(row - 1, cols - 1);
        }
    }

    pub fn show_cursor(&mut self) {
        self.styles.cursor_state.hidden = false;
    }

    pub fn hide_cursor(&mut self) {
        self.styles.cursor_state.hidden = true;
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor_pos = self.cursor_pos;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_pos = self.saved_cursor_pos;
    }
}
