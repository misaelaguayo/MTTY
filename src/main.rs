extern crate sdl2;

mod animal;
pub mod backend;
mod commands;
mod config;
pub mod frontend;
mod term;

fn main() -> Result<(), String> {
    let config = config::Config::new();

    let mut terminal = term::Terminal::build(config);
    terminal.run();

    Ok(())
}
