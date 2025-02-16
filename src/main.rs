use std::{
    fs::File,
    io::{self, Read},
    os::fd::{AsFd, AsRawFd, FromRawFd},
    thread,
    time::Duration,
};

mod term;

use macroquad::prelude::*;

#[macroquad::main("MTTY")]
async fn main() {
    let term = term::Term::new();

    let mut master_file = unsafe { File::from_raw_fd(term.parent.as_raw_fd()) };
    let stdin = io::stdin();
    let mut buffer = [0u8; 1024];

    let mut read_fds = nix::sys::select::FdSet::new();
    read_fds.insert(term.parent.as_fd());
    read_fds.insert(stdin.as_fd());
    nix::sys::select::select(None, &mut read_fds, None, None, None).unwrap();

    loop {
        clear_background(BLACK);

        // get_last_key_pressed().map(|key| {
        //     draw_text(&format!("{:?}", key), 20.0, 20.0, 30.0, DARKGRAY);
        // });

        if read_fds.contains(stdin.as_fd()) {
            let bytes_read = io::stdin().read(&mut buffer).unwrap_or(0);
            if bytes_read == 0 {
                break;
            }
            draw_text(
                &String::from_utf8_lossy(&buffer[..bytes_read]),
                20.0,
                20.0,
                30.0,
                DARKGRAY,
            );
        }
        if read_fds.contains(term.parent.as_fd()) {
            let bytes_read = master_file.read(&mut buffer).unwrap_or(0);
            if bytes_read == 0 {
                break;
            }
            draw_text(
                &String::from_utf8_lossy(&buffer[..bytes_read]),
                20.0,
                20.0,
                30.0,
                DARKGRAY,
            );
        }

        next_frame().await;
        thread::sleep(Duration::from_millis(10000));
    }
}
