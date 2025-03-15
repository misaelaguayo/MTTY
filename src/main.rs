use std::{
    os::fd::{AsFd, AsRawFd},
    thread,
};

use term::{read_from_raw_fd, write_to_fd};

pub mod term;

fn main() {
    let term = term::Term::new().unwrap();
    let read_raw_fd = term.parent.as_raw_fd();
    let write_fd = term.parent.as_fd();

    thread::spawn(move || loop {
        if let Some(data) = read_from_raw_fd(read_raw_fd) {
            print!("{}", String::from_utf8(data).unwrap());
        }
    });

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        write_to_fd(write_fd, input.as_bytes());
    }
}
