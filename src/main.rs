use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use commands::Command;
use config::Config;
use iced::{
    widget::{button, column, text, Column, Row},
    Element, Theme,
};
use tokio::sync::broadcast;

pub mod commands;
pub mod config;
pub mod fonts;
pub mod grid;
pub mod statemachine;
pub mod styles;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = config::Config::default();

    // Flag set when ui is closed and signals background threads to exit
    let exit_flag = Arc::new(AtomicBool::new(false));

    let term = term::Term::new(&config)?;
    let read_fd = term.parent.try_clone()?;
    let write_fd = term.parent.try_clone()?;

    let (output_tx, output_rx_ui) = broadcast::channel(10000);

    let (input_tx, input_rx): (broadcast::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>) =
        broadcast::channel(10000);

    term::spawn_read_thread(read_fd.as_raw_fd(), exit_flag.clone(), output_tx);
    term::spawn_write_thread(write_fd, input_rx, exit_flag.clone());

    let res = start_ui(&config, exit_flag, input_tx, output_rx_ui);
    if let Err(e) = res {
        eprintln!("Error starting UI: {}", e);
    }

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: Arc<AtomicBool>,
    tx: broadcast::Sender<Vec<u8>>,
    rx_ui: broadcast::Receiver<Command>,
) -> iced::Result {
    let app = iced::application("title", App::update, App::view);

    app.run()
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    Update,
}

struct App {
    grid: Vec<Vec<char>>,
}

impl Default for App {
    fn default() -> Self {
        let grid = vec![vec!['.'; 80]; 24]; // Initialize a grid of 80x24 with spaces
        App { grid }
    }
}

impl App {
    fn view(&self) -> iced::Element<Message> {
        let mut rows: Vec<Element<Message>> = self
            .grid
            .iter()
            .map(|row| {
                Row::from_vec(row.iter().map(|&c| text(c.to_string()).into()).collect()).into()
            })
            .collect();

        rows.push(
            Row::new()
                .push(button("Exit").on_press(Message::Update))
                .into(),
        );

        Column::from_vec(rows).into()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Init => {
                println!("UI initialized");
            }
            Message::Update => {
                self.grid[0][0] = 'X'; // Example update to the grid
            }
        }
    }
}
