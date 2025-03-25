pub struct Config {
    pub width: f32,
    pub height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
        }
    }
}
