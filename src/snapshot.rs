use crate::grid::{Cell, Grid};
use crate::styles::{Color, CursorState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSnapshot {
    pub version: String,
    pub timestamp: String,
    pub width: u16,
    pub height: u16,
    pub cursor_pos: (usize, usize),
    pub saved_cursor_pos: (usize, usize),
    pub scroll_pos: usize,
    pub scroll_region: (usize, usize),
    pub alternate_active: bool,
    pub cursor_state: CursorState,
    pub active_fg: Color,
    pub active_bg: Color,
    pub cells: Vec<Cell>,
}

impl TerminalSnapshot {
    pub fn from_grid(grid: &Grid) -> Self {
        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            width: grid.width,
            height: grid.height,
            cursor_pos: grid.cursor_pos,
            saved_cursor_pos: grid.saved_cursor_pos,
            scroll_pos: grid.scroll_pos,
            scroll_region: grid.get_scroll_region(),
            alternate_active: grid.is_alternate(),
            cursor_state: grid.styles.cursor_state,
            active_fg: grid.styles.active_text_color,
            active_bg: grid.styles.active_background_color,
            cells: grid.active_grid_ref().clone(),
        }
    }

    pub fn save_to_file(&self, path: &PathBuf) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// Get the debug output directory, creating it if it doesn't exist
pub fn get_debug_dir() -> io::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find config directory"))?;
    let debug_dir = config_dir.join("mtty").join("debug");
    fs::create_dir_all(&debug_dir)?;
    Ok(debug_dir)
}

/// Generate a timestamped filename for snapshots
pub fn snapshot_filename() -> String {
    let now: DateTime<Utc> = Utc::now();
    format!("snapshot_{}.json", now.format("%Y%m%d_%H%M%S"))
}

/// Generate a timestamped filename for recordings
pub fn recording_filename() -> String {
    let now: DateTime<Utc> = Utc::now();
    format!("recording_{}.json", now.format("%Y%m%d_%H%M%S"))
}

/// Take a snapshot and save it to the debug directory
pub fn take_snapshot(grid: &Grid) -> io::Result<PathBuf> {
    let debug_dir = get_debug_dir()?;
    let filename = snapshot_filename();
    let path = debug_dir.join(filename);

    let snapshot = TerminalSnapshot::from_grid(grid);
    snapshot.save_to_file(&path)?;

    log::info!("Snapshot saved to: {:?}", path);
    Ok(path)
}
