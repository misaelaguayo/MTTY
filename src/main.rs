use std::{thread, time::Duration};
mod term;


use macroquad::prelude::*;

#[macroquad::main("MTTY")]
async fn main() {
    let term = term::Term::new();
    println!("Starting");
    thread::sleep(Duration::from_secs(1));

    // start user shell
    term.write(b"bash\n");

    loop {
        clear_background(BLACK);

        match term.read() {
            Some(data) => {
                let text = String::from_utf8(data).unwrap();
                draw_text(&text, 20.0, 20.0, 30.0, DARKGRAY);
            }
            None => {}
        }

        next_frame().await;
        thread::sleep(Duration::from_millis(100));
    }
}
