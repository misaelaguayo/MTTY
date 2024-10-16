use std::{env, io::{Error, ErrorKind, Read}};

use crate::term::Command;

fn read_shell_command(command: Command, path: String) -> Result<Vec<u8>, Error> {
    let mut handle = std::process::Command::new(path)
        .arg("-c")
        .arg(format!("{} {}", command.command, command.args.join(" ")))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn().unwrap();

    match handle.try_wait() {
        Ok(Some(status)) => {
            println!("Command exited with status: {}", status);
            if status.success() {
                let output = handle.wait_with_output().unwrap();
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(stdout.as_bytes().to_vec())
            } else {
                let output = handle.wait_with_output().unwrap();
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(Error::new(ErrorKind::Other, stderr.to_string()))
            }
        }
        Ok(None) => {
            match handle.stdout {
                Some(stdout) => {
                    let mut output = vec![];
                    for b in stdout.bytes() {
                        match b {
                            Ok(b) => output.push(b),
                            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
                        }
                    }

                    Ok(output)
                }
                None => Ok(vec![]),
            }
        }
        Err(e) => Err(Error::new(ErrorKind::Other, e)),
    }
}

pub fn read_command(command: Command) -> Result<Vec<u8>, Error> {
    let shell = env::var("SHELL").unwrap();
    match read_shell_command(command, shell) {
        Ok(output) => Ok(output),
        Err(e) => Err(Error::new(ErrorKind::Other, e)),
    }
}
