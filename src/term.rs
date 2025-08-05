use std::env;
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{
    io::Error,
    os::fd::{BorrowedFd, OwnedFd},
    process::{Child, Command},
};

use nix::libc::{self, c_int, TIOCSCTTY};
use nix::unistd::read;
use nix::unistd::write;
use rustix::termios::{self, OptionalActions, Termios};
use rustix_openpty::openpty;
use tokio::sync::broadcast::{self, Receiver};

use crate::commands::{ClientCommand as TermCommand, ServerCommand};
use crate::config::Config;
use crate::statemachine;

use vte::ansi::Processor;

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
        Ok(size) => {
            println!("Wrote {} bytes", size);
        }
        Err(e) => eprintln!("Failed to write to file: {:?}", e),
    }
}

pub struct Term {
    pub parent: OwnedFd,
    pub child: Child,
}

fn set_controlling_terminal(fd: c_int) {
    let res = unsafe {
        #[allow(clippy::cast_lossless)]
        libc::ioctl(fd, TIOCSCTTY as _, 0)
    };

    if res < 0 {
        panic!(
            "Failed to set controlling terminal: {}",
            Error::last_os_error()
        );
    }
}

impl Term {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let winsize = termios::Winsize {
            ws_row: config.rows - 1,
            ws_col: config.cols - 1,
            ws_xpixel: config.width as u16,
            ws_ypixel: config.height as u16,
        };

        let pty = openpty(None, Some(&winsize)).expect("Failed to open pty");
        let (master, slave) = (pty.controller, pty.user);

        Self::from_fd(master, slave)
    }

    fn from_fd(master: OwnedFd, slave: OwnedFd) -> Result<Term, Error> {
        let master_fd = master.as_raw_fd();
        let slave_fd = slave.as_raw_fd();
        if let Ok(mut termios) = termios::tcgetattr(&master) {
            enable_raw_mode(&mut termios);

            termios.input_modes.insert(termios::InputModes::IUTF8);

            let _ = termios::tcsetattr(&master, OptionalActions::Now, &termios);
        }

        let mut builder = Self::default_shell_command();

        builder.stdin(slave.try_clone()?);
        builder.stdout(slave.try_clone()?);
        builder.stderr(slave);

        unsafe {
            builder.pre_exec(move || {
                // Create a new process group.
                let err = libc::setsid();
                if err == -1 {
                    panic!(
                        "Failed to create new process group: {}",
                        Error::last_os_error()
                    );
                }

                set_controlling_terminal(slave_fd);

                // No longer need slave/master fds.
                libc::close(slave_fd);
                libc::close(master_fd);

                libc::signal(libc::SIGCHLD, libc::SIG_DFL);
                libc::signal(libc::SIGHUP, libc::SIG_DFL);
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                libc::signal(libc::SIGTERM, libc::SIG_DFL);
                libc::signal(libc::SIGALRM, libc::SIG_DFL);

                Ok(())
            });
        }

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

        let user = env::var("USER").expect("Failed to get user");

        let mut command = Command::new("/usr/bin/login");

        let exec = format!("exec -l {}", shell);
        command.args([
            "-q",
            "-flp",
            &user,
            "/bin/bash",
            "--noprofile",
            "--norc",
            "-c",
            &exec,
        ]);
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
    output_tx: broadcast::Sender<TermCommand>,
) {
    tokio::spawn(async move {
        let mut processor: Processor = Processor::new();
        let mut statemachine = statemachine::StateMachine::new(output_tx);

        loop {
            if let Some(data) = read_from_raw_fd(fd) {
                processor.advance(&mut statemachine, &data);
            }

            if read_exit_flag.load(Ordering::Relaxed) {
                break;
            }
        }
    });
}

pub fn spawn_write_thread(
    write_fd: OwnedFd,
    mut input_rx: Receiver<ServerCommand>,
    exit_flag: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        loop {
            match input_rx.recv().await {
                Ok(ServerCommand::RawData(data)) => {
                    write_to_fd(write_fd.as_fd(), &data);
                }
                _ => {}
            }

            if exit_flag.load(Ordering::Relaxed) {
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
