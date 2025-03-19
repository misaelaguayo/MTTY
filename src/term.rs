use std::env;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{
    io::Error,
    os::fd::{BorrowedFd, OwnedFd},
    process::{Child, Command},
};

use nix::libc::{self, c_int};
use nix::unistd::read;
use nix::unistd::write;
use rustix::termios::{self, OptionalActions, Termios};
use rustix_openpty::openpty;
use tokio::sync::mpsc;

use crate::commands::Command as TermCommand;
use crate::statemachine;

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
    pub parent: OwnedFd,
    pub child: Child,
}

impl Term {
    pub fn new() -> Result<Self, Error> {
        let pty = openpty(None, None).expect("Failed to open pty");
        let (master, slave) = (pty.controller, pty.user);

        Self::from_fd(master, slave)
    }

    fn from_fd(master: OwnedFd, slave: OwnedFd) -> Result<Term, Error> {
        let master_fd = master.as_raw_fd();
        if let Ok(mut termios) = termios::tcgetattr(&master) {
            enable_raw_mode(&mut termios);

            // set read timeout
            termios.special_codes[termios::SpecialCodeIndex::VTIME] = 1;

            // set read minimum bytes
            termios.special_codes[termios::SpecialCodeIndex::VMIN] = 0;

            let _ = termios::tcsetattr(&master, OptionalActions::Now, &termios);
        }

        let mut builder = Self::default_shell_command();

        builder.stdin(slave.try_clone()?);
        builder.stdout(slave.try_clone()?);
        builder.stderr(slave);

        match builder.spawn() {
            Ok(child) => {
                unsafe {
                    // this allows read to return immediately and not block drawing
                    set_nonblocking(master_fd);
                }
                Ok(Term {
                    parent: master,
                    child,
                })
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(target_os = "macos")]
    fn default_shell_command() -> Command {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_name = shell.split('/').last().unwrap();

        let user = env::var("USER").expect("Failed to get user");

        let mut command = Command::new("/usr/bin/login");

        let exec = format!("exec -a {} {}", shell_name, shell);
        command.args(["-flp", &user, shell_name, "-fc", &exec]);
        command
    }
}

fn enable_raw_mode(termios: &mut Termios) {
    termios.input_modes.remove(
        termios::InputModes::BRKINT
            | termios::InputModes::ICRNL
            | termios::InputModes::INPCK
            | termios::InputModes::ISTRIP
            | termios::InputModes::IXON,
    );
    termios.output_modes.remove(termios::OutputModes::OPOST);
    termios.local_modes.remove(
        termios::LocalModes::ECHO
            | termios::LocalModes::ICANON
            | termios::LocalModes::ISIG
            | termios::LocalModes::IEXTEN,
    );
    termios.control_modes.remove(termios::ControlModes::CS8);
}

pub fn spawn_read_thread(
    fd: i32,
    read_exit_flag: Arc<AtomicBool>,
    output_tx: mpsc::Sender<TermCommand>,
) {
    tokio::spawn(async move {
        let mut statemachine = vte::Parser::new();
        let mut performer = statemachine::StateMachine::new(output_tx);

        loop {
            if let Some(data) = read_from_raw_fd(fd) {
                statemachine.advance(&mut performer, &data);
            }

            if read_exit_flag.load(Ordering::Relaxed) {
                break;
            }
        }
    });
}

unsafe fn set_nonblocking(fd: c_int) {
    use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};

    let res = fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) | O_NONBLOCK);
    assert_eq!(res, 0);
}
