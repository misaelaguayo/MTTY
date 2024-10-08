use std::env;

use crate::term::Command;

pub fn ls() -> Result<Vec<String>, std::io::Error> {
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

pub fn cd(path: String) -> Result<String, std::io::Error> {
    env::set_current_dir(path)?;

    Ok(env::current_dir()?.display().to_string())
}

pub fn pwd() -> Result<String, std::io::Error> {
    Ok(env::current_dir()?.display().to_string())
}

pub fn whoami() -> Result<String, std::io::Error> {
    let username = env::var("USER");

    match username {
        Ok(username) => Ok(username),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

pub fn read_command(command: Command) -> Result<Vec<String>, std::io::Error> {
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
        _ => Ok(vec![format!("Unknown command: {}", command.command)]),
    }
}
