use commands::ClientCommand;
use config::Config;
use std::io::Write;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{
    commands::ServerCommand,
    ui::{Runner, WgpuRunner},
};

pub mod app;
pub mod commands;
pub mod config;
pub mod fonts;
pub mod grid;
pub mod renderer;
pub mod statemachine;
pub mod styles;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} - [{}] {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .init();

    let app = app::App::new(Config::load(), Arc::new(AtomicBool::new(false)));

    start_ui(
        &app.config,
        &app.is_running,
        &app.server_channel.input_transmitter,
        &app.client_channel.output_receiver,
    );

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: &Arc<AtomicBool>,
    tx: &Sender<ServerCommand>,
    ui_update_receiver: &Receiver<ClientCommand>,
) {
    let runner = WgpuRunner {
        exit_flag: exit_flag.clone(),
        config: config.clone(),
        tx: tx.clone(),
        rx: ui_update_receiver.resubscribe(),
    };

    runner.run();
}
