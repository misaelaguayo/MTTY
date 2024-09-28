extern crate sdl2;

pub mod backend;
mod commands;
mod config;
pub mod frontend;
mod term;

fn main() -> Result<(), String> {
    let config = config::Config::new();

    let terminal = term::Terminal::build(config);
    terminal.run();

    Ok(())
}
