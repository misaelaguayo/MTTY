use crate::{
    commands::SgrAttribute,
    config::Config,
    styles::{Color, Styles},
};

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

impl ToString for Cell {
    fn to_string(&self) -> String {
        String::from(self.char)
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

    pub fn pretty_print(&self) {
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

        for row in &self.cells {
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
        if row >= self.cells.len() {
            self.add_rows(row - self.cells.len() + 1);
            self.scroll_pos = row;
        }

        self.cursor_pos = (row, col);
    }

    pub fn add_rows(&mut self, rows: usize) {
        let cols = self.width;
        self.cells.resize_with(self.cells.len() + rows, || {
            vec![
                Cell::new(
                    ' ',
                    self.styles.active_text_color,
                    self.styles.active_background_color
                );
                cols as usize
            ]
        });
    }

    pub fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (mut row, mut col) = self.cursor_pos;

        if col >= cols as usize - 1 {
            self.set_pos(row + 1, 0);
        }

        (row, col) = self.cursor_pos;

        match c {
            '\n' => {
                self.set_pos(row + 1, 0);
            }
            '\r' => {
                self.set_pos(row, 0);
            }
            _ => {
                self.cells[row][col] = Cell {
                    char: c,
                    fg: self.styles.active_text_color,
                    bg: self.styles.active_background_color,
                    attrs: vec![],
                };
                self.set_pos(row, col + 1);
            }
        }
    }

    pub fn clear_screen(&mut self) {
        self.cells = vec![
            vec![
                Cell::new(
                    ' ',
                    self.styles.active_text_color,
                    self.styles.active_background_color
                );
                self.cells[0].len()
            ];
            self.cells.len()
        ];
    }

    pub fn delete_character(&mut self) {
        let (mut row, mut col) = self.cursor_pos;
        let cols = self.cells[0].len() as usize;

        if col > 0 {
            (row, col) = self.cursor_pos;
            self.cells[row][col] = Cell::new(
                ' ',
                self.styles.active_text_color,
                self.styles.active_background_color,
            );

            self.set_pos(row, col - 1);
        } else if row > 0 {
            (row, col) = self.cursor_pos;
            self.cells[row][col] = Cell::new(
                ' ',
                self.styles.active_text_color,
                self.styles.active_background_color,
            );

            self.set_pos(row - 1, cols - 1);
        } else {
            self.cells[row][col] = Cell::new(
                ' ',
                self.styles.active_text_color,
                self.styles.active_background_color,
            );
        }
    }

    pub fn show_cursor(&mut self) {}

    pub fn save_cursor(&mut self) {
        self.saved_cursor_pos = self.cursor_pos;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_pos = self.saved_cursor_pos;
    }
}
