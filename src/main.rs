extern crate sdl2;

use std::env;
use std::path::Path;
use term::setup_font;

mod term;

// fn run(terminal: Terminal){
//     terminal.frontend
// }

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    println!("linked sdl2_ttf: {}", sdl2::ttf::get_linked_version());

    if args.len() < 2 {
        println!("Usage: ./demo font.[ttf|ttc|fon]")
    } else {
        let path: &Path = Path::new(&args[1]);
        setup_font(path)?;
        // run();
    }

    Ok(())
}
