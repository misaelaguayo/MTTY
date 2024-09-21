extern crate sdl2;

mod animal;
mod term;

fn main() -> Result<(), String> {
    println!("linked sdl2_ttf: {}", sdl2::ttf::get_linked_version());

    let mut terminal = term::Terminal::new();
    terminal.run();

    Ok(())
}
