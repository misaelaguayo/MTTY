use config::Config;
use std::io::Write;
use std::sync::RwLock;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::broadcast::{Receiver, Sender};

use crate::commands::ServerCommand;

pub mod app;
pub mod commands;
pub mod config;
pub mod fonts;
mod graphics;
pub mod grid;
pub mod statemachine;
pub mod styles;
pub mod term;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let config = Config::default();
    let shared_config = Arc::new(RwLock::new(config));

    graphics::window::display_grid(shared_config)?;

    // let app = app::App::new(Config::default(), Arc::new(AtomicBool::new(false)));
    //
    // start_ui(
    //     &app.config,
    //     &app.is_running,
    //     &app.server_channel.input_transmitter,
    //     &app.client_channel.output_receiver,
    // );
    //
    Ok(())
}

// fn start_ui(
//     config: &Config,
//     exit_flag: &Arc<AtomicBool>,
//     tx: &Sender<ServerCommand>,
//     ui_update_receiver: &Receiver<ClientCommand>,
// ) {
//     let runner = EguiRunner {
//         exit_flag: exit_flag.clone(),
//         config: config.clone(),
//         tx: tx.clone(),
//         rx: ui_update_receiver.resubscribe(),
//     };
//
//     runner.run();
// }
