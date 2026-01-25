use crate::commands::ClientCommand;
use crate::grid::Grid;
use crate::snapshot::{get_debug_dir, recording_filename, TerminalSnapshot};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub command: ClientCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub version: String,
    pub initial_state: TerminalSnapshot,
    pub events: Vec<RecordedEvent>,
    pub final_state: Option<TerminalSnapshot>,
}

impl Recording {
    pub fn new(initial_state: TerminalSnapshot) -> Self {
        Self {
            version: "1.0".to_string(),
            initial_state,
            events: Vec::new(),
            final_state: None,
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

/// Active recording session
pub struct Recorder {
    recording: Recording,
    start_time: Instant,
    sequence: u64,
}

impl Recorder {
    pub fn new(grid: &Grid) -> Self {
        let initial_state = TerminalSnapshot::from_grid(grid);
        Self {
            recording: Recording::new(initial_state),
            start_time: Instant::now(),
            sequence: 0,
        }
    }

    pub fn record_command(&mut self, command: &ClientCommand) {
        let event = RecordedEvent {
            sequence: self.sequence,
            timestamp_ms: self.start_time.elapsed().as_millis() as u64,
            command: command.clone(),
        };
        self.recording.events.push(event);
        self.sequence += 1;
    }

    pub fn finish(mut self, grid: &Grid) -> io::Result<PathBuf> {
        self.recording.final_state = Some(TerminalSnapshot::from_grid(grid));

        let debug_dir = get_debug_dir()?;
        let filename = recording_filename();
        let path = debug_dir.join(filename);

        self.recording.save_to_file(&path)?;
        log::info!("Recording saved to: {:?}", path);
        log::info!("Recorded {} events", self.recording.events.len());

        Ok(path)
    }

    pub fn event_count(&self) -> usize {
        self.recording.events.len()
    }
}

/// Playback controller for stepping through recordings
pub struct Player {
    recording: Recording,
    current_index: usize,
}

impl Player {
    pub fn new(recording: Recording) -> Self {
        Self {
            recording,
            current_index: 0,
        }
    }

    pub fn load_from_file(path: &PathBuf) -> io::Result<Self> {
        let recording = Recording::load_from_file(path)?;
        log::info!("Loaded recording with {} events", recording.events.len());
        Ok(Self::new(recording))
    }

    /// Get the initial snapshot to restore grid state
    pub fn initial_state(&self) -> &TerminalSnapshot {
        &self.recording.initial_state
    }

    /// Step forward one event, returns the command to apply
    pub fn step_forward(&mut self) -> Option<&ClientCommand> {
        if self.current_index < self.recording.events.len() {
            let event = &self.recording.events[self.current_index];
            self.current_index += 1;
            Some(&event.command)
        } else {
            None
        }
    }

    /// Step backward one event, returns the index we moved to
    /// Note: To actually go back, you need to replay from initial state
    pub fn step_backward(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    /// Get current event without advancing
    pub fn current_event(&self) -> Option<&RecordedEvent> {
        if self.current_index < self.recording.events.len() {
            Some(&self.recording.events[self.current_index])
        } else {
            None
        }
    }

    /// Get the event at a specific index
    pub fn event_at(&self, index: usize) -> Option<&RecordedEvent> {
        self.recording.events.get(index)
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Jump to a specific position
    pub fn seek(&mut self, index: usize) {
        self.current_index = index.min(self.recording.events.len());
    }

    /// Current position in the recording
    pub fn position(&self) -> usize {
        self.current_index
    }

    /// Total number of events
    pub fn total_events(&self) -> usize {
        self.recording.events.len()
    }

    /// Check if at the end
    pub fn is_finished(&self) -> bool {
        self.current_index >= self.recording.events.len()
    }

    /// Get all events up to current position (for replaying to a point)
    pub fn events_up_to_current(&self) -> &[RecordedEvent] {
        &self.recording.events[..self.current_index]
    }
}
