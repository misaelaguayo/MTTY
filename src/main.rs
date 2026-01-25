use clap::Parser;
use commands::ClientCommand;
use config::Config;
use std::io::Write;
use std::path::PathBuf;
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
pub mod recording;
pub mod renderer;
pub mod snapshot;
pub mod statemachine;
pub mod styles;
pub mod term;
pub mod ui;

#[derive(Parser, Debug, Clone)]
#[command(name = "mtty")]
#[command(about = "A GPU-accelerated terminal emulator")]
pub struct Args {
    /// Replay a recording file instead of starting a normal terminal session
    #[arg(long, value_name = "FILE")]
    pub replay: Option<PathBuf>,
}

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

    let args = Args::parse();

    if let Some(replay_path) = args.replay {
        // Replay mode - no PTY, just playback
        start_replay_ui(&Config::load(), &replay_path);
    } else {
        // Normal terminal mode
        let app = app::App::new(Config::load(), Arc::new(AtomicBool::new(false)));

        start_ui(
            &app.config,
            &app.is_running,
            &app.server_channel.input_transmitter,
            &app.client_channel.output_receiver,
        );
    }

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: &Arc<AtomicBool>,
    tx: &Sender<ServerCommand>,
    ui_update_receiver: &Receiver<ClientCommand>,
) {
    let runner = WgpuRunner::new(
        exit_flag.clone(),
        config.clone(),
        tx.clone(),
        ui_update_receiver.resubscribe(),
        None,
    );

    runner.run();
}

fn start_replay_ui(config: &Config, replay_path: &PathBuf) {
    use crate::recording::Player;

    let player = match Player::load_from_file(replay_path) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to load recording: {}", e);
            eprintln!("Failed to load recording from {:?}: {}", replay_path, e);
            return;
        }
    };

    let exit_flag = Arc::new(AtomicBool::new(false));
    // Create dummy channels for replay mode (won't be used)
    let (tx, _) = tokio::sync::broadcast::channel::<ServerCommand>(1);
    let (_, rx) = tokio::sync::broadcast::channel::<ClientCommand>(1);

    let runner = WgpuRunner::new(
        exit_flag,
        config.clone(),
        tx,
        rx,
        Some(player),
    );

    runner.run();
}
