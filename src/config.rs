#[derive(Clone)]
pub struct Config {
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub rows: u16,
    pub cols: u16,
}

impl Default for Config {
    fn default() -> Self {
        const WIDTH: f32 = 640.0;
        const HEIGHT: f32 = 480.0;
        const FONT_SIZE: f32 = 16.0;

        // Cell dimensions based on font size (monospace: width ~0.6x, height ~1.2x)
        let cell_width = FONT_SIZE * 0.6;
        let cell_height = FONT_SIZE * 1.2;
        let cols = (WIDTH / cell_width).floor() as u16;
        let rows = (HEIGHT / cell_height).floor() as u16;

        Self {
            width: WIDTH,
            height: HEIGHT,
            font_size: FONT_SIZE,
            rows,
            cols,
        }
    }
}

impl Config {
    pub fn get_col_rows_from_size(&self, width: f32, height: f32) -> (u16, u16) {
        // Cell dimensions based on font size (monospace: width ~0.6x, height ~1.2x)
        let cell_width = self.font_size * 0.6;
        let cell_height = self.font_size * 1.2;
        let cols = (width / cell_width).floor() as u16;
        let rows = (height / cell_height).floor() as u16;
        (cols, rows)
    }
}
