use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use commands::Command;
use config::Config;
use iced::{
    widget::{button, text, Column, Row},
    Element, Task,
};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::grid::Grid;

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

    let res = start_ui(config, exit_flag, input_tx, output_rx_ui);
    if let Err(e) = res {
        eprintln!("Error starting UI: {}", e);
    }

    Ok(())
}

fn start_ui(
    config: Config,
    exit_flag: Arc<AtomicBool>,
    tx: broadcast::Sender<Vec<u8>>,
    rx_ui: broadcast::Receiver<Command>,
) -> iced::Result {
    println!("Starting UI...");
    iced::application("Title", App::update, App::view)
        .run_with(move || App::new(&config, exit_flag, tx, rx_ui))
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    Update,
}

struct App {
    exit_flag: Arc<AtomicBool>,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
    grid: Grid,
}

impl App {
    fn handle_command(&mut self, command: Command) {
        let cols = self.grid.width;
        match command {
            Command::Print(c) => {
                self.grid.place_character_in_grid(cols, c);
            }
            _ => {}
        }
    }

    fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<Vec<u8>>,
        rx: Receiver<Command>,
    ) -> (Self, Task<Message>) {
        println!("Creating new App instance...");

        (
            Self {
                exit_flag,
                input: String::new(),
                tx,
                rx,
                grid: Grid::new(config),
            },
            Task::done(Message::Init),
        )
    }

    fn view(&self) -> iced::Element<Message> {
        println!("Rendering UI...");
        let mut rows: Vec<Element<Message>> = self
            .grid
            .read_active_grid()
            .iter()
            .map(|row| {
                Row::from_vec(row.iter().map(|c| text(c.to_string()).into()).collect()).into()
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
        println!("Updating UI with message: {:?}", message);
        match message {
            Message::Init => {
                println!("UI initialized");
            }
            Message::Update => {
                while let Ok(command) = self.rx.try_recv() {
                    self.handle_command(command);
                }
            }
        }
    }
}
