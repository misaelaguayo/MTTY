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
    cells: Vec<Cell>,
    alternate_screen: Vec<Cell>,
    alternate: bool,
    pub width: u16,
    pub height: u16,
    pub cursor_pos: (usize, usize),
    pub saved_cursor_pos: (usize, usize),
    pub scroll_pos: usize,
    pub styles: Styles,
    /// Row-level dirty tracking - each element indicates if that row needs re-rendering
    dirty_rows: Vec<bool>,
    /// Previous cursor position for tracking cursor movement
    prev_cursor_pos: (usize, usize),
}

impl Grid {
    pub fn new(config: &Config) -> Self {
        let width = config.cols;
        let height = config.rows;
        let cells = vec![Cell::default(); (width as usize) * (height as usize)];
        let alternate_screen = vec![Cell::default(); (width as usize) * (height as usize)];
        // Start with all rows dirty to force initial render
        let dirty_rows = vec![true; height as usize];

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
            dirty_rows,
            prev_cursor_pos: (0, 0),
        }
    }

    /// Returns true if any row has changed since last clear
    pub fn is_dirty(&self) -> bool {
        self.dirty_rows.iter().any(|&d| d)
    }

    /// Returns the dirty state of all rows
    pub fn dirty_rows(&self) -> &[bool] {
        &self.dirty_rows
    }

    /// Clears all dirty flags (call after rendering)
    pub fn clear_dirty(&mut self) {
        for dirty in &mut self.dirty_rows {
            *dirty = false;
        }
        self.prev_cursor_pos = self.cursor_pos;
    }

    /// Marks a specific row as dirty
    fn mark_row_dirty(&mut self, row: usize) {
        // Convert absolute row to dirty_rows index based on current scroll position
        let start_row = self.scroll_pos.saturating_sub(self.height as usize - 1);
        if row >= start_row {
            let display_row = row - start_row;
            if display_row < self.dirty_rows.len() {
                self.dirty_rows[display_row] = true;
            }
        }
    }

    /// Marks all rows as dirty (for operations like screen clear, resize, swap)
    fn mark_all_dirty(&mut self) {
        for dirty in &mut self.dirty_rows {
            *dirty = true;
        }
    }

    pub fn active_grid(&mut self) -> &mut Vec<Cell> {
        if self.alternate {
            &mut self.alternate_screen
        } else {
            &mut self.cells
        }
    }

    /// Get a read-only reference to the active grid
    pub fn active_grid_ref(&self) -> &Vec<Cell> {
        if self.alternate {
            &self.alternate_screen
        } else {
            &self.cells
        }
    }

    /// Check if currently using alternate screen
    pub fn is_alternate(&self) -> bool {
        self.alternate
    }

    pub fn swap_active_grid(&mut self) {
        self.alternate = !self.alternate;
        // Reset scroll position when switching screens
        self.scroll_pos = self.height as usize - 1;
        self.mark_all_dirty();
    }

    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        self.width = new_cols;
        self.height = new_rows;

        let new_size = (new_cols as usize) * (new_rows as usize);

        // Clear and reinitialize both screens with new dimensions
        self.cells = vec![Cell::default(); new_size];
        self.alternate_screen = vec![Cell::default(); new_size];

        // Resize dirty_rows to match new height
        self.dirty_rows = vec![true; new_rows as usize];

        // Reset positions
        self.scroll_pos = new_rows as usize - 1;
        self.cursor_pos = (0, 0);
    }

    pub fn pretty_print(&mut self) {
        log::info!("Grid: {}x{}", self.width, self.height);
        log::info!("Cursor Position: {:?}", self.cursor_pos);
        log::info!("Saved Cursor Position: {:?}", self.saved_cursor_pos);
        log::info!("Scroll Position: {:?}", self.scroll_pos);
        log::info!(
            "Active Grid: {:?}",
            if self.alternate { "Alternate" } else { "Main" }
        );
        log::info!(
            "Colors: Bg: {:?}, Fg: {:?}",
            self.styles.active_background_color,
            self.styles.active_text_color
        );

        for row in 0..self.height as usize {
            let start = row * self.width as usize;
            let end = start + self.width as usize;
            let row_cells = &self.active_grid()[start..end];
            let row_str: String = row_cells
                .iter()
                .map(|cell| if cell.char == ' ' { '.' } else { cell.char })
                .collect();
            log::info!("{}", row_str);
        }
    }

    pub fn set_pos(&mut self, row: usize, col: usize) {
        let rows = self.active_grid().len();
        if row >= rows {
            log::warn!(
                "Row {} exceeds grid height {}. Adjusting to last row.",
                row,
                rows - 1
            );
            self.add_rows(row - rows + 1);
            self.scroll_pos = row;
        }

        // Mark old cursor row as dirty (to redraw without cursor)
        let old_row = self.cursor_pos.0;
        self.mark_row_dirty(old_row);

        self.cursor_pos = (row, col);

        // Mark new cursor row as dirty (to draw cursor at new position)
        self.mark_row_dirty(row);
    }

    pub fn add_rows(&mut self, rows: usize) {
        let cols = self.width;
        // Apply reverse video mode - swap fg and bg
        let (fg, bg) = if self.styles.reverse {
            (
                self.styles.active_background_color,
                self.styles.active_text_color,
            )
        } else {
            (
                self.styles.active_text_color,
                self.styles.active_background_color,
            )
        };

        for _ in 0..rows {
            for _ in 0..cols {
                self.active_grid().push(Cell::new(' ', fg, bg));
            }
        }
        // Adding rows typically means scrolling, mark all visible rows dirty
        self.mark_all_dirty();
    }

    pub fn place_character_in_grid(&mut self, cols: u16, c: char) {
        let (mut row, mut col) = self.cursor_pos;

        if col >= cols as usize {
            self.set_pos(row + 1, 0);
        }

        (row, col) = self.cursor_pos;
        // Apply reverse video mode - swap fg and bg
        let (fg, bg) = if self.styles.reverse {
            (
                self.styles.active_background_color,
                self.styles.active_text_color,
            )
        } else {
            (
                self.styles.active_text_color,
                self.styles.active_background_color,
            )
        };

        match c {
            '\n' => {
                self.set_pos(row + 1, 0);
            }
            '\r' => {
                self.set_pos(row, 0);
            }
            _ => {
                // Calculate the index in the flat vector
                let index = row * (self.width as usize) + col;
                let active_grid_len = self.active_grid().len();
                if index >= active_grid_len {
                    self.add_rows(row - (active_grid_len / (self.width as usize)) + 1);
                }
                self.active_grid()[index] = Cell::new(c, fg, bg);
                // Mark the specific row as dirty
                self.mark_row_dirty(row);
                self.set_pos(row, col + 1);
            }
        }
    }

    pub fn clear_screen(&mut self) {
        // Apply reverse video mode - swap fg and bg
        let (fg, bg) = if self.styles.reverse {
            (
                self.styles.active_background_color,
                self.styles.active_text_color,
            )
        } else {
            (
                self.styles.active_text_color,
                self.styles.active_background_color,
            )
        };

        // Clear out any rows which may have been added
        let rows = self.height as usize;
        let cols = self.width as usize;
        self.active_grid().truncate(rows * cols);

        let active_grid_len = self.active_grid().len();
        for i in 0..active_grid_len {
            self.active_grid()[i] = Cell::new(' ', fg, bg);
        }

        self.scroll_pos = 0;
        self.cursor_pos = (0, 0);
        self.mark_all_dirty();
    }

    pub fn delete_character(&mut self) {
        let (row, col) = self.cursor_pos;
        let cols = self.width as usize;
        // Apply reverse video mode - swap fg and bg
        let (fg, bg) = if self.styles.reverse {
            (
                self.styles.active_background_color,
                self.styles.active_text_color,
            )
        } else {
            (
                self.styles.active_text_color,
                self.styles.active_background_color,
            )
        };

        let index = row * (self.width as usize) + col;
        if index < self.active_grid().len() {
            self.active_grid()[index] = Cell::new(' ', fg, bg);
        }

        // Mark current row dirty
        self.mark_row_dirty(row);

        if col > 0 {
            self.set_pos(row, col - 1);
        } else if row > 0 {
            self.set_pos(row - 1, cols - 1);
        }
    }

    pub fn show_cursor(&mut self) {
        self.styles.cursor_state.hidden = false;
        self.mark_row_dirty(self.cursor_pos.0);
    }

    pub fn hide_cursor(&mut self) {
        self.styles.cursor_state.hidden = true;
        self.mark_row_dirty(self.cursor_pos.0);
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor_pos = self.cursor_pos;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_pos = self.saved_cursor_pos;
    }
}
