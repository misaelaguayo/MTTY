use std::{env, io::{Error, ErrorKind}};

use crate::term::Command;

fn read_shell_command(command: Command, path: String) -> Result<Vec<String>, Error> {
    let output = match std::process::Command::new(path)
        .arg("-c")
        .arg(command.command)
        .args(command.args)
        .output()
    {
        Ok(output) => output,
        Err(e) => return Err(Error::new(ErrorKind::Other, e)),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(vec![stdout.to_string(), stderr.to_string()])
}

pub fn read_command(command: Command) -> Result<Vec<String>, Error> {
    let shell = env::var("SHELL").unwrap();
    match read_shell_command(command, shell) {
        Ok(output) => Ok(output),
        Err(e) => Err(Error::new(ErrorKind::Other, e)),
    }
}
