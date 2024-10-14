use std::{env, io::{Error, ErrorKind}};

use crate::term::Command;

pub fn ls() -> Result<Vec<String>, Error> {
    let path = env::current_dir()?;
    let paths = std::fs::read_dir(path)?;

    let mut files = Vec::new();
    for path in paths {
        let path = path?.path();
        let path = path.to_str().unwrap().to_string();
        files.push(path);
    }

    Ok(files)
}

pub fn cd(path: String) -> Result<String, Error> {
    env::set_current_dir(path)?;

    Ok(env::current_dir()?.display().to_string())
}

pub fn pwd() -> Result<String, Error> {
    Ok(env::current_dir()?.display().to_string())
}

pub fn whoami() -> Result<String, Error> {
    let username = env::var("USER");

    match username {
        Ok(username) => Ok(username),
        Err(e) => Err(Error::new(ErrorKind::Other, e)),
    }
}

pub fn read_command(command: Command) -> Result<Vec<String>, Error> {
    match command.command.to_lowercase().as_str() {
        "ls" => {
            let files = ls()?;
            Ok(files)
        }
        "pwd" => {
            let path = pwd()?;
            Ok(vec![path])
        }
        "cd" => {
            if command.args.len() > 1 {
                return Ok(vec![format!("Too many arguments for cd")]);
            }

            let path = cd(command.args[0].clone())?;
            Ok(vec![path])
        }
        "whoami" => {
            let username = whoami()?;
            Ok(vec![username])
        }
        _ => {
            let output = match std::process::Command::new(command.command)
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
    }
}
