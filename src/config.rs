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
        const WIDTH: f32 = 1280.0;
        const HEIGHT: f32 = 960.0;
        const FONT_SIZE: f32 = 16.0;

        let rows = (HEIGHT / FONT_SIZE).floor() as u16;
        let cols = (WIDTH / FONT_SIZE).floor() as u16;

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
        let cols = (width / self.font_size).floor() as u16;
        let rows = (height / self.font_size).floor() as u16;
        (cols, rows)
    }
}
