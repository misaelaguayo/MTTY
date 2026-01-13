/// Calculate cell dimensions based on font size
/// Returns (cell_width, cell_height)
pub fn get_cell_size(font_size: f32) -> (f32, f32) {
    // For monospace fonts, width is approximately 0.6 of the font size
    // Height includes line spacing (1.2x font size)
    let cell_width = font_size * 0.6;
    let cell_height = font_size * 1.2;
    (cell_width, cell_height)
}
