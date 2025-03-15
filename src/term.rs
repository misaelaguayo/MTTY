use std::{
    fs::File,
    io::Error,
    os::fd::{BorrowedFd, OwnedFd},
    process::{Child, Command},
};

use nix::unistd::read;
use nix::unistd::write;
use rustix::termios::{self, OptionalActions};
use rustix_openpty::openpty;

// Steps to create a terminal
// Call openpty to get a master and slave fd
// The master fd is used to read and write to the terminal
// The slave fd is used to create a new process
//
// Once we have the master and slave fd, we fork a new process
// In the child process, we create a new process with the user's default shell
// We then set the child process's stdin, stdout, and stderr to the slave fd
// This is done by calling dup2(slave_fd, STDIN_FILENO), dup2(slave_fd, STDOUT_FILENO), and
// dup2(slave_fd, STDERR_FILENO)
// We should also call setsid to make the child process the session leader
// This allows the child process to have a controlling terminal and handle signals
//
// on the parent process, we close the slave fd and set the terminal attributes
// Example terminal attributes that should be set are terminal size, turn off echo, turn off
// canonical mode, etc
// We will then poll the master fd for any data
// This can be done by calling read(master_fd, buffer)
// We can also use syscalls like select or poll to wait for data on the master fd
//
pub fn read_from_raw_fd(fd: i32) -> Option<Vec<u8>> {
    let mut read_buffer = [0; 65536];

    let read_result = read(fd, &mut read_buffer);

    match read_result {
        Ok(bytes_read) => Some(read_buffer[..bytes_read].to_vec()),
        Err(_e) => None,
    }
}

pub fn write_to_fd(fd: BorrowedFd, data: &[u8]) {
    let write_result = write(fd, data);

    match write_result {
        Ok(_) => (),
        Err(e) => eprintln!("Failed to write to file: {:?}", e),
    }
}

pub struct Term {
    pub parent: File,
    pub child: Child,
}

impl Term {
    pub fn new() -> Result<Self, Error> {
        let pty = openpty(None, None).expect("Failed to open pty");
        let (master, slave) = (pty.controller, pty.user);

        Self::from_fd(master, slave)
    }

    fn from_fd(master: OwnedFd, slave: OwnedFd) -> Result<Term, Error> {
        if let Ok(mut termios) = termios::tcgetattr(&master) {
            termios.local_modes.set(termios::LocalModes::ECHO, false);

            let _ = termios::tcsetattr(&master, OptionalActions::Now, &termios);
        }

        let mut builder = Self::default_shell_command();

        builder.stdin(slave.try_clone()?);
        builder.stdout(slave.try_clone()?);
        builder.stderr(slave);

        match builder.spawn() {
            Ok(child) => Ok(Term {
                parent: File::from(master),
                child,
            }),
            Err(e) => Err(e),
        }
    }

    #[cfg(target_os = "macos")]
    fn default_shell_command() -> Command {
        // TODO: Grab shell from environment variable

        let mut command = Command::new("/usr/bin/login");

        let exec = format!("exec -a {} {}", "zsh", "/bin/zsh");
        command.args(["-flp", "misaelaguayo", "/bin/zsh", "-fc", &exec]);
        command
    }
}
