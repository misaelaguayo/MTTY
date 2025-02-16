use std::{
    io::{self},
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    process,
};

use nix::{
    libc::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO},
    pty::openpty,
    sys::termios,
    unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult},
};

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

fn set_terminal_attrs(fd: BorrowedFd) {
    if let Ok(termios) = termios::tcgetattr(io::stdin().as_fd()) {
        let _ = termios::tcsetattr(fd, termios::SetArg::TCSANOW, &termios);
    }
}

impl Term {
    pub fn new() -> Term {
        let pty = openpty(None, None).expect("Failed to open pty");
        let master_fd = pty.master;
        let slave_fd = pty.slave;

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // close(master_fd.as_raw_fd()).unwrap();
                setsid().unwrap();
                dup2(slave_fd.as_raw_fd(), STDIN_FILENO).unwrap();
                dup2(slave_fd.as_raw_fd(), STDOUT_FILENO).unwrap();
                dup2(slave_fd.as_raw_fd(), STDERR_FILENO).unwrap();
                // close(slave_fd.as_raw_fd()).unwrap();
                let _ = execvp(
                    &std::ffi::CString::new("/bin/zsh").unwrap(),
                    &[std::ffi::CString::new("zsh").unwrap()],
                );
                // process::exit(1);
            }
            Ok(ForkResult::Parent { .. }) => {
                // close(slave_fd.as_raw_fd()).unwrap();
                set_terminal_attrs(master_fd.as_fd());
            }
            Err(_) => {
                eprintln!("Fork failed");
                process::exit(1);
            }
        }

        Term {
            parent: master_fd,
            child: slave_fd.as_raw_fd(),
        }
    }

    pub fn write(&self, data: &[u8]) {
        write_to_fd(self.parent.as_fd(), data);
    }

    pub fn read(&self) -> Option<Vec<u8>> {
        read_from_fd(self.parent.as_raw_fd())
    }
}
