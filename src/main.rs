use std::thread;

extern crate sdl2;

pub mod backend;
mod commands;
mod config;
pub mod frontend;
mod term;

fn main() -> Result<(), String> {
    let config = config::Config::new();

    let mut terminal = term::Terminal::build(config);
    let mut backend = dyn_clone::clone_box(&*terminal.backend);
    thread::spawn(move || {
        backend.execute();
    });

    terminal.run();

    Ok(())
}
