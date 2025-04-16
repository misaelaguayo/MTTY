use eframe::{
    egui::{self, FontFamily, FontId, TextStyle},
    epaint::text::{FontInsert, InsertFontFamily},
};

use crate::config::Config;

fn add_font(ctx: &egui::Context) {
    ctx.add_font(FontInsert::new(
        "hack-font",
        egui::FontData::from_static(include_bytes!("../assets/Hack-Regular.ttf")),
        vec![
            InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            },
            InsertFontFamily {
                family: egui::FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
        ],
    ));
}

fn replace_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "hack-font".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Hack-Regular.ttf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "hack-font".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("hack-font".to_owned());

    ctx.set_fonts(fonts);
}

pub fn configure_text_styles(ctx: &egui::Context, config: &Config) {
    use FontFamily::Proportional;
    use TextStyle::*;

    replace_fonts(ctx);
    add_font(ctx);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (Heading, FontId::new(config.font_size + 2.0, Proportional)),
        (Body, FontId::new(config.font_size, Proportional)),
        (Monospace, FontId::new(config.font_size, Proportional)),
        (Button, FontId::new(config.font_size, Proportional)),
        (Small, FontId::new(config.font_size - 2.0, Proportional)),
    ]
    .into();
    ctx.set_style(style);
}
