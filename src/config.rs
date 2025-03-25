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
        const FONT_SIZE: f32 = 12.0;

        let rows = (HEIGHT / FONT_SIZE) as u16;
        let cols = (WIDTH / FONT_SIZE) as u16;

        Self {
            width: WIDTH,
            height: HEIGHT,
            font_size: FONT_SIZE,
            rows,
            cols,
        }
    }
}
