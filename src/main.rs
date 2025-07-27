use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use commands::Command;
use config::Config;
use iced::{
    stream,
    widget::{text, Column, Row},
    Element, Subscription, Task,
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
    iced::application("Title", App::update, App::view)
        .subscription(App::subscription)
        .run_with(move || App::new(&config, exit_flag, tx, rx_ui))
}

#[derive(Debug)]
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
            Command::Put(c) => {
                self.tx
                    .send(vec![c as u8])
                    .expect("Failed to send character");
            }
            Command::Backspace => {
                self.tx.send(vec![8]).expect("Failed to send backspace");
            }
            _ => {}
        }
    }

    fn new(
        config: &Config,
        exit_flag: Arc<AtomicBool>,
        tx: Sender<Vec<u8>>,
        rx: Receiver<Command>,
    ) -> (Self, Task<Command>) {
        (
            Self {
                exit_flag,
                input: String::new(),
                tx,
                rx,
                grid: Grid::new(config),
            },
            Task::none(),
        )
    }

    fn view(&self) -> iced::Element<Command> {
        let rows: Vec<Element<Command>> = self
            .grid
            .read_active_grid()
            .iter()
            .map(|row| {
                Row::from_vec(row.iter().map(|c| text(c.to_string()).into()).collect()).into()
            })
            .collect();

        Column::from_vec(rows).into()
    }

    fn update(&mut self, command: Command) {
        self.handle_command(command);
    }

    fn subscription(&self) -> Subscription<Command> {
        let mut rx = self.rx.resubscribe();

        let client_subscription = iced::event::listen_with(|event, _status, _id| match event {
            iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key, text, ..
            }) => {
                if let Some(t) = text {
                    return Some(Command::Put(t.to_string().chars().next().unwrap()));
                }

                match key {
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace) => {
                        return Some(Command::Backspace);
                    }
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                        return Some(Command::NewLine);
                    }
                    _ => {}
                }

                None
            }
            _ => None,
        });

        let server_subscription = Subscription::run_with_id(
            "server",
            stream::channel(10000, |mut tx| async move {
                while let Ok(command) = rx.recv().await {
                    tx.try_send(command).expect("Failed to send command");
                }
            }),
        );

        Subscription::batch(vec![client_subscription, server_subscription])
    }
}
