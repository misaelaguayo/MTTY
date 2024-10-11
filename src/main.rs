use std::thread;
// use font_kit::source::SystemSource;

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

    test();
    Ok(())
}

// a test function used to test random stuff
fn test() {
    // print all available font families
    // let system_source = SystemSource::new();
    // let families = system_source.all_families().unwrap();
    // for family in families {
    //     println!("{}", family);
    // }
}
