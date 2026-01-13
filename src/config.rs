use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

/// TOML configuration file structure
#[derive(Deserialize, Default)]
struct ConfigFile {
    window: Option<WindowConfig>,
    font: Option<FontConfig>,
    shell: Option<ShellConfig>,
}

#[derive(Deserialize)]
struct WindowConfig {
    width: Option<f32>,
    height: Option<f32>,
}

#[derive(Deserialize)]
struct FontConfig {
    size: Option<f32>,
    family: Option<String>,
}

#[derive(Deserialize)]
struct ShellConfig {
    program: Option<String>,
    args: Option<Vec<String>>,
}

/// Runtime configuration
#[derive(Clone)]
pub struct Config {
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub rows: u16,
    pub cols: u16,
    pub shell: String,
    pub shell_args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        const WIDTH: f32 = 640.0;
        const HEIGHT: f32 = 480.0;
        const FONT_SIZE: f32 = 16.0;

        // Cell dimensions based on font size (monospace: width ~0.6x, height ~1.2x)
        let cell_width = FONT_SIZE * 0.6;
        let cell_height = FONT_SIZE * 1.2;
        let cols = (WIDTH / cell_width).floor() as u16;
        let rows = (HEIGHT / cell_height).floor() as u16;

        // Default shell based on platform
        #[cfg(target_os = "macos")]
        let default_shell = "/bin/zsh".to_string();
        #[cfg(target_os = "linux")]
        let default_shell = "/bin/bash".to_string();
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let default_shell = "/bin/sh".to_string();

        let shell = env::var("SHELL").unwrap_or(default_shell);

        Self {
            width: WIDTH,
            height: HEIGHT,
            font_size: FONT_SIZE,
            font_family: None, // Use system monospace font by default
            rows,
            cols,
            shell,
            shell_args: vec!["-l".to_string()], // Login shell by default
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults
    pub fn load() -> Self {
        let mut config = Config::default();

        if let Some(config_path) = Self::config_path() {
            if config_path.exists() {
                match fs::read_to_string(&config_path) {
                    Ok(contents) => match toml::from_str::<ConfigFile>(&contents) {
                        Ok(file_config) => {
                            config.apply_file_config(file_config);
                            log::info!("Loaded config from {:?}", config_path);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse config file: {}", e);
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to read config file: {}", e);
                    }
                }
            } else {
                log::info!(
                    "No config file found at {:?}, using defaults",
                    config_path
                );
            }
        }

        config
    }

    /// Get the config file path (~/.config/mtty/config.toml)
    fn config_path() -> Option<PathBuf> {
        // first try to get from XDG_CONFIG_HOME
        if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
            let mut path = PathBuf::from(xdg_config_home);
            path.push("mtty");
            path.push("config.toml");
            return Some(path);
        }

        // fallback to ~/.config
        dirs::config_dir().map(|mut path| {
            path.push("mtty");
            path.push("config.toml");
            path
        })
    }

    /// Apply settings from the config file
    fn apply_file_config(&mut self, file_config: ConfigFile) {
        // Window settings
        if let Some(window) = file_config.window {
            if let Some(width) = window.width {
                self.width = width;
            }
            if let Some(height) = window.height {
                self.height = height;
            }
        }

        // Font settings
        if let Some(font) = file_config.font {
            if let Some(size) = font.size {
                self.font_size = size;
            }
            if let Some(family) = font.family {
                self.font_family = Some(family);
            }
        }

        // Shell settings
        if let Some(shell) = file_config.shell {
            if let Some(program) = shell.program {
                self.shell = program;
            }
            if let Some(args) = shell.args {
                self.shell_args = args;
            }
        }

        // Recalculate rows/cols based on updated dimensions
        let cell_width = self.font_size * 0.6;
        let cell_height = self.font_size * 1.2;
        self.cols = (self.width / cell_width).floor() as u16;
        self.rows = (self.height / cell_height).floor() as u16;
    }

    pub fn get_col_rows_from_size(&self, width: f32, height: f32) -> (u16, u16) {
        // Cell dimensions based on font size (monospace: width ~0.6x, height ~1.2x)
        let cell_width = self.font_size * 0.6;
        let cell_height = self.font_size * 1.2;
        let cols = (width / cell_width).floor() as u16;
        let rows = (height / cell_height).floor() as u16;
        (cols, rows)
    }
}
