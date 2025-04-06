use eframe::egui::Color32;

pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    pub fn to_color32(&self) -> Color32 {
        match self {
            Color::Black => Color32::BLACK,
            Color::Red => Color32::RED,
            Color::Green => Color32::GREEN,
            Color::Yellow => Color32::YELLOW,
            Color::Blue => Color32::BLUE,
            Color::Magenta => Color32::from_rgb(0, 111, 184),
            Color::Cyan => Color32::from_rgb(111, 38, 113),
            Color::White => Color32::WHITE,
            Color::Gray => Color32::GRAY,
            Color::BrightRed => Color32::from_rgb(255, 0, 0),
            Color::BrightGreen => Color32::from_rgb(0, 255, 0),
            Color::BrightYellow => Color32::from_rgb(255, 255, 0),
            Color::BrightBlue => Color32::from_rgb(0, 0, 255),
            Color::BrightMagenta => Color32::from_rgb(255, 0, 255),
            Color::BrightCyan => Color32::from_rgb(0, 255, 255),
            Color::BrightWhite => Color32::from_rgb(255, 255, 255),
        }
    }
}

pub struct Styles {
    pub background_color: Color,
    pub text_color: Color,
    pub font_size: u32,
    pub italic: bool,
    pub underline: bool,
}

impl Styles {
    pub fn default() -> Self {
        Self {
            background_color: Color::Black,
            text_color: Color::White,
            font_size: 16,
            italic: false,
            underline: false,
        }
    }

    pub fn set_foreground_color_from_int(&mut self, color: i16) {
        match color {
            30 => self.text_color = Color::Black,
            31 => self.text_color = Color::Red,
            32 => self.text_color = Color::Green,
            33 => self.text_color = Color::Yellow,
            34 => self.text_color = Color::Blue,
            35 => self.text_color = Color::Magenta,
            36 => self.text_color = Color::Cyan,
            37 => self.text_color = Color::White,
            90 => self.text_color = Color::Gray,
            91 => self.text_color = Color::BrightRed,
            92 => self.text_color = Color::BrightGreen,
            93 => self.text_color = Color::BrightYellow,
            94 => self.text_color = Color::BrightBlue,
            95 => self.text_color = Color::BrightMagenta,
            96 => self.text_color = Color::BrightCyan,
            97 => self.text_color = Color::BrightWhite,
            _ => {
                // Not supported
            }
        }
    }

    // TODO: Implement a method to apply styles to the UI
    // pub fn from_config(config: &Config) -> Self {
    //     Self::new(
    //         config.background_color.clone(),
    //         config.text_color.clone(),
    //         config.font_size,
    //     )
    // }
}
