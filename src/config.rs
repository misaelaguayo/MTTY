use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub font_path: String,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Config {
    pub fn new() -> Config {
        let maybe_path = search_for_config_paths();
        if let Some(path) = maybe_path {
            return Config::from_path(&path);
        }

        Config {
            font_path: String::from("fonts/Arial-Unicode.ttf"),
            screen_width: 800,
            screen_height: 600,
        }
    }

    fn from_path(path: &str) -> Config {
        let contents = std::fs::read_to_string(path).unwrap();
        let deserialized: Config = toml::from_str(&contents).unwrap();
        deserialized
    }
}

pub fn search_for_config_paths() -> Option<String> {
    let mut paths = Vec::new();
    paths.push(String::from("config.toml"));
    paths.push(String::from("~/.config/mtty/config.toml"));
    
    for path in paths {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}
