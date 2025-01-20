use std::{os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd}, thread};

use nix::{pty::openpty, unistd::{read, write}};

fn read_from_fd(fd: RawFd) -> Option<Vec<u8>> {
    let mut read_buffer = [0; 65536];
    let read_result = read(fd, &mut read_buffer);
    match read_result {
        Ok(bytes_read) => Some(read_buffer[..bytes_read].to_vec()),
        Err(_e) => None,
    }
}

fn write_to_fd(fd: BorrowedFd, data: &[u8]) {
    let write = write(fd, data);
    match write {
        Ok(bytes_written) => println!("Wrote {} bytes", bytes_written),
        Err(e) => println!("Error writing to fd: {}", e),
    }
}

pub struct Term {
    pub parent: OwnedFd,
    pub child: RawFd,
}

impl Term {
    pub fn new() -> Term {
        let res = openpty(None, None).expect("Failed to open pty");
        // write_to_fd(res.master.as_fd(), b"bash\n");
        // thread::sleep(std::time::Duration::from_secs(1));
        // match read_from_fd(res.master.as_raw_fd()) {
        //     Some(data) => {
        //         let text = String::from_utf8(data).unwrap();
        //         println!("Read from pty: {}", text);
        //     }
        //     None => println!("Failed to read from pty"),
        // }

        println!("Created pty with master fd: {} and slave fd: {}", res.master.as_raw_fd(), res.slave.as_raw_fd());

        Term {
            parent: res.master,
            child: res.slave.as_raw_fd(),
        }
    }

    pub fn write(&self, data: &[u8]) {
        write_to_fd(self.parent.as_fd(), data);
    }

    pub fn read(&self) -> Option<Vec<u8>> {
        read_from_fd(self.parent.as_raw_fd())
    }
}
