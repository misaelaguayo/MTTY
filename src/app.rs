use crate::commands::{ClientCommand, ServerCommand};
use crate::config::Config;
use crate::term::Term;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct ClientChannel {
    pub output_transmitter: broadcast::Sender<ClientCommand>,
    pub output_receiver: broadcast::Receiver<ClientCommand>,
}

pub struct ServerChannel {
    pub input_transmitter: broadcast::Sender<ServerCommand>,
    pub input_receiver: broadcast::Receiver<ServerCommand>,
}

pub struct App {
    pub config: Config,
    pub is_running: Arc<AtomicBool>,
    pub term: Term,
    pub client_channel: ClientChannel,
    pub server_channel: ServerChannel,
}

impl App {
    pub fn new(config: Config, is_running: Arc<AtomicBool>) -> Self {
        let (output_tx, output_rx): (
            broadcast::Sender<ClientCommand>,
            broadcast::Receiver<ClientCommand>,
        ) = broadcast::channel(10000);

        let (input_tx, input_rx): (
            broadcast::Sender<ServerCommand>,
            broadcast::Receiver<ServerCommand>,
        ) = broadcast::channel(10000);

        let client_channel = ClientChannel {
            output_transmitter: output_tx,
            output_receiver: output_rx,
        };

        let server_channel = ServerChannel {
            input_transmitter: input_tx,
            input_receiver: input_rx,
        };

        let term = Term::new(&config).expect("Failed to create terminal");

        term.init(is_running.clone(), &client_channel, &server_channel);

        App {
            config,
            is_running,
            term,
            client_channel,
            server_channel,
        }
    }
}
