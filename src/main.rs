extern crate sdl2;

mod animal;
mod config;
mod term;

fn main() -> Result<(), String> {
    println!("linked sdl2_ttf: {}", sdl2::ttf::get_linked_version());

    let config = config::Config::new();

    let mut terminal = term::Terminal::build(config);
    terminal.run();

    Ok(())
}
