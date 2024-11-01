use std::{
    env,
    io::{Error, ErrorKind, Read},
};

use log::info;

use crate::term::{Command, CommandOutputIterator};

impl CommandOutputIterator {
    fn new(command: Command, path: String) -> Result<Self, Error> {
        let mut handle = std::process::Command::new(path)
            .arg("-c")
            .arg(format!("{} {}", command.command, command.args.join(" ")))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        match handle.try_wait() {
            Ok(Some(status)) => {
                info!("Command exited with status: {}", status);
                if status.success() {
                    let output = handle.wait_with_output().unwrap();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(CommandOutputIterator {
                        output: stdout.as_bytes().to_vec(),
                    })
                } else {
                    let output = handle.wait_with_output().unwrap();
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(Error::new(ErrorKind::Other, stderr.to_string()))
                }
            }
            Ok(None) => match handle.stdout {
                Some(stdout) => Ok(CommandOutputIterator {
                    output: stdout.bytes().map(|b| b.unwrap()).collect(),
                }),
                None => Ok(CommandOutputIterator { output: vec![] }),
            },
            Err(_) => Err(Error::new(ErrorKind::Other, "Error")),
        }
    }
}

impl Iterator for CommandOutputIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.output.is_empty() {
            None
        } else {
            Some(self.output.remove(0))
        }
    }
}

fn read_shell_command(command: Command, path: String) -> Result<CommandOutputIterator, Error> {
    CommandOutputIterator::new(command, path)
}

pub fn read_command(command: Command) -> Result<CommandOutputIterator, Error> {
    let shell = env::var("SHELL").unwrap();
    match read_shell_command(command, shell) {
        Ok(output) => Ok(output),
        Err(e) => Err(Error::new(ErrorKind::Other, e)),
    }
}
