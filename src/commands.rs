use std::env;

pub fn ls_command() -> Result<Vec<String>, std::io::Error> {
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

pub fn read_command(command: String) -> Result<Vec<String>, std::io::Error> {
    match command.to_lowercase().as_str() {
        "ls" => {
            let files = ls_command()?;
            Ok(files)
        }
        _ => Ok(vec![format!("Unknown command: {}", command)]),
    }
}
