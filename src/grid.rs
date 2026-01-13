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
    /// Count of dirty rows for O(1) is_dirty() check
    dirty_count: usize,
    /// Previous cursor position for tracking cursor movement
    prev_cursor_pos: (usize, usize),
    /// Scrolling region (top row, bottom row) - 0-indexed, inclusive
    scroll_region: (usize, usize),
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
            dirty_count: height as usize, // All rows start dirty
            prev_cursor_pos: (0, 0),
            scroll_region: (0, height as usize - 1),
        }
    }

    /// Returns true if any row has changed since last clear (O(1))
    pub fn is_dirty(&self) -> bool {
        self.dirty_count > 0
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
        self.dirty_count = 0;
        self.prev_cursor_pos = self.cursor_pos;
    }

    /// Marks a specific row as dirty
    fn mark_row_dirty(&mut self, row: usize) {
        // Convert absolute row to dirty_rows index based on current scroll position
        let start_row = self.scroll_pos.saturating_sub(self.height as usize - 1);
        if row >= start_row {
            let display_row = row - start_row;
            if display_row < self.dirty_rows.len() && !self.dirty_rows[display_row] {
                self.dirty_rows[display_row] = true;
                self.dirty_count += 1;
            }
        }
    }

    /// Marks all rows as dirty (for operations like screen clear, resize, swap)
    fn mark_all_dirty(&mut self) {
        for dirty in &mut self.dirty_rows {
            if !*dirty {
                *dirty = true;
                self.dirty_count += 1;
            }
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

        // Resize dirty_rows to match new height (all dirty)
        self.dirty_rows = vec![true; new_rows as usize];
        self.dirty_count = new_rows as usize;

        // Reset positions and scroll region
        self.scroll_pos = new_rows as usize - 1;
        self.cursor_pos = (0, 0);
        self.scroll_region = (0, new_rows as usize - 1);
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
        let grid_rows = self.active_grid().len() / self.width as usize;
        if row >= grid_rows {
            log::debug!("Row {} exceeds grid rows {}. Adding rows.", row, grid_rows);
            self.add_rows(row - grid_rows + 1);
        }

        // Mark old cursor row as dirty (to redraw without cursor)
        let old_row = self.cursor_pos.0;
        self.mark_row_dirty(old_row);

        self.cursor_pos = (row, col);

        // Auto-scroll: if cursor is below visible area, scroll to follow
        if row > self.scroll_pos {
            self.scroll_pos = row;
            self.mark_all_dirty(); // Need to redraw all rows when scrolling
        }

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

    /// Set the scrolling region (1-indexed from terminal, converted to 0-indexed)
    pub fn set_scroll_region(&mut self, top: usize, bottom: Option<usize>) {
        // Terminal uses 1-indexed, convert to 0-indexed
        let top = top.saturating_sub(1);
        let bottom = bottom.map(|b| b.saturating_sub(1)).unwrap_or(self.height as usize - 1);
        self.scroll_region = (top, bottom.min(self.height as usize - 1));
        // Move cursor to home position when scroll region is set
        self.set_pos(0, 0);
    }

    /// Scroll content up within the scroll region (content moves up, blank lines appear at bottom)
    pub fn scroll_up(&mut self, count: usize) {
        let (top, bottom) = self.scroll_region;
        let width = self.width as usize;
        let region_height = bottom - top + 1;

        if count >= region_height {
            // Clear the entire scroll region
            self.clear_scroll_region();
            return;
        }

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();

        // Move rows up within the region
        for row in top..(bottom - count + 1) {
            let dest_idx = row * width;
            let src_idx = (row + count) * width;
            for col in 0..width {
                if src_idx + col < grid.len() && dest_idx + col < grid.len() {
                    grid[dest_idx + col] = grid[src_idx + col].clone();
                }
            }
        }

        // Clear the bottom rows
        for row in (bottom - count + 1)..=bottom {
            let start_idx = row * width;
            for col in 0..width {
                if start_idx + col < grid.len() {
                    grid[start_idx + col] = Cell::new(' ', fg, bg);
                }
            }
        }

        self.mark_all_dirty();
    }

    /// Scroll content down within the scroll region (content moves down, blank lines appear at top)
    pub fn scroll_down(&mut self, count: usize) {
        let (top, bottom) = self.scroll_region;
        let width = self.width as usize;
        let region_height = bottom - top + 1;

        if count >= region_height {
            // Clear the entire scroll region
            self.clear_scroll_region();
            return;
        }

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();

        // Move rows down within the region (iterate from bottom to avoid overwriting)
        for row in ((top + count)..=bottom).rev() {
            let dest_idx = row * width;
            let src_idx = (row - count) * width;
            for col in 0..width {
                if src_idx + col < grid.len() && dest_idx + col < grid.len() {
                    grid[dest_idx + col] = grid[src_idx + col].clone();
                }
            }
        }

        // Clear the top rows
        for row in top..(top + count) {
            let start_idx = row * width;
            for col in 0..width {
                if start_idx + col < grid.len() {
                    grid[start_idx + col] = Cell::new(' ', fg, bg);
                }
            }
        }

        self.mark_all_dirty();
    }

    /// Clear the entire scroll region
    fn clear_scroll_region(&mut self) {
        let (top, bottom) = self.scroll_region;
        let width = self.width as usize;

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();
        for row in top..=bottom {
            let start_idx = row * width;
            for col in 0..width {
                if start_idx + col < grid.len() {
                    grid[start_idx + col] = Cell::new(' ', fg, bg);
                }
            }
        }

        self.mark_all_dirty();
    }

    /// Insert blank lines at cursor position within scroll region
    pub fn insert_blank_lines(&mut self, count: usize) {
        let (row, _) = self.cursor_pos;
        let (top, bottom) = self.scroll_region;
        let width = self.width as usize;

        // Only operate if cursor is within scroll region
        if row < top || row > bottom {
            return;
        }

        let region_height = bottom - row + 1;
        let count = count.min(region_height);

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();

        // Move rows down from cursor position (iterate from bottom to avoid overwriting)
        for dest_row in ((row + count)..=bottom).rev() {
            let src_row = dest_row - count;
            let dest_idx = dest_row * width;
            let src_idx = src_row * width;
            for col in 0..width {
                if src_idx + col < grid.len() && dest_idx + col < grid.len() {
                    grid[dest_idx + col] = grid[src_idx + col].clone();
                }
            }
        }

        // Clear the rows at cursor position
        for r in row..(row + count).min(bottom + 1) {
            let start_idx = r * width;
            for col in 0..width {
                if start_idx + col < grid.len() {
                    grid[start_idx + col] = Cell::new(' ', fg, bg);
                }
            }
        }

        self.mark_all_dirty();
    }

    /// Delete lines at cursor position within scroll region (content moves up)
    pub fn delete_lines(&mut self, count: usize) {
        let (row, _) = self.cursor_pos;
        let (top, bottom) = self.scroll_region;
        let width = self.width as usize;

        // Only operate if cursor is within scroll region
        if row < top || row > bottom {
            return;
        }

        let region_height = bottom - row + 1;
        let count = count.min(region_height);

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();

        // Move rows up from cursor position
        for src_row in (row + count)..=bottom {
            let dest_row = src_row - count;
            let dest_idx = dest_row * width;
            let src_idx = src_row * width;
            for col in 0..width {
                if src_idx + col < grid.len() && dest_idx + col < grid.len() {
                    grid[dest_idx + col] = grid[src_idx + col].clone();
                }
            }
        }

        // Clear the bottom rows
        for r in (bottom - count + 1)..=bottom {
            let start_idx = r * width;
            for col in 0..width {
                if start_idx + col < grid.len() {
                    grid[start_idx + col] = Cell::new(' ', fg, bg);
                }
            }
        }

        self.mark_all_dirty();
    }

    /// Reverse index - move cursor up one line, scroll down if at top of scroll region
    pub fn reverse_index(&mut self) {
        let (row, col) = self.cursor_pos;
        let (top, _) = self.scroll_region;

        if row == top {
            // At top of scroll region, scroll content down
            self.scroll_down(1);
        } else if row > 0 {
            // Move cursor up
            self.set_pos(row - 1, col);
        }
    }

    /// Insert blank characters at cursor, shifting existing chars to the right
    pub fn insert_blanks(&mut self, count: usize) {
        let (row, col) = self.cursor_pos;
        let width = self.width as usize;

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();
        let row_start = row * width;
        let row_end = row_start + width;

        // Shift characters to the right
        for i in (col + count..width).rev() {
            let dest_idx = row_start + i;
            let src_idx = row_start + i - count;
            if dest_idx < row_end && src_idx < row_end && dest_idx < grid.len() && src_idx < grid.len() {
                grid[dest_idx] = grid[src_idx].clone();
            }
        }

        // Fill with blanks at cursor position
        for i in col..(col + count).min(width) {
            let idx = row_start + i;
            if idx < grid.len() {
                grid[idx] = Cell::new(' ', fg, bg);
            }
        }

        self.mark_row_dirty(row);
    }

    /// Delete characters at cursor, shifting remaining chars left
    pub fn delete_chars(&mut self, count: usize) {
        let (row, col) = self.cursor_pos;
        let width = self.width as usize;

        let (fg, bg) = if self.styles.reverse {
            (self.styles.active_background_color, self.styles.active_text_color)
        } else {
            (self.styles.active_text_color, self.styles.active_background_color)
        };

        let grid = self.active_grid();
        let row_start = row * width;

        // Shift characters to the left
        for i in col..(width - count) {
            let dest_idx = row_start + i;
            let src_idx = row_start + i + count;
            if dest_idx < grid.len() && src_idx < grid.len() {
                grid[dest_idx] = grid[src_idx].clone();
            }
        }

        // Fill end of line with blanks
        for i in (width - count)..width {
            let idx = row_start + i;
            if idx < grid.len() {
                grid[idx] = Cell::new(' ', fg, bg);
            }
        }

        self.mark_row_dirty(row);
    }
}
