use std::os::fd::{AsFd, AsRawFd};

use term::{read_from_raw_fd, write_to_fd};
use tokio::sync::mpsc;

pub mod term;

#[tokio::main]
async fn main() {
    let term = term::Term::new().unwrap();
    let read_raw_fd = term.parent.try_clone().unwrap();
    let write_fd = term.parent.try_clone().unwrap();
    let (tx, _rx) = mpsc::channel(1);

    tokio::spawn(async move {
        loop {
            if let Some(data) = read_from_raw_fd(read_raw_fd.as_raw_fd()) {
                if let Err(_) = tx.send(data).await {
                    break;
                }
            }
        }
    });

    tokio::spawn(async move {
        loop {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();

            write_to_fd(write_fd.as_fd(), input.as_bytes());
        }
    });

    // Window::new("MTTY", draw(rx));
}

// async fn draw(mut rx: mpsc::Receiver<Vec<u8>>) {
//     let mut term_output = String::new();
//     loop {
//         clear_background(WHITE);
//
//         if let Some(data) = rx.recv().await {
//             let data = String::from_utf8(data).unwrap();
//             term_output.push_str(&data);
//         }
//
//         draw_text(&term_output, 20.0, 20.0, 30.0, DARKGRAY);
//
//         next_frame().await;
//     }
// }
