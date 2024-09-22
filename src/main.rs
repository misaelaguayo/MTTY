extern crate sdl2;

mod animal;
mod config;
mod term;

fn main() -> Result<(), String> {
    let config = config::Config::new();

    let mut terminal = term::Terminal::build(config);
    terminal.run();

    Ok(())
}
