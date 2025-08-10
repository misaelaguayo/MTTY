use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use commands::ClientCommand;
use config::Config;
use eframe::egui::{self};
use tokio::sync::broadcast;

use crate::commands::ServerCommand;

pub mod app;
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
    env_logger::init();

    let app = app::App::new(Config::default(), Arc::new(AtomicBool::new(false)));

    start_ui(
        &app.config,
        app.is_running,
        app.server_channel.input_transmitter.clone(),
        app.client_channel.output_receiver.resubscribe(),
    );

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: Arc<AtomicBool>,
    tx: broadcast::Sender<ServerCommand>,
    rx_ui: broadcast::Receiver<ClientCommand>,
) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([config.width, config.height]),
        ..Default::default()
    };

    let rx_remote = rx_ui.resubscribe();
    let redraw_exit_flag = exit_flag.clone();

    let _ = eframe::run_native(
        "MTTY",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            fonts::configure_text_styles(&ctx, &config);
            tokio::spawn(async move {
                redraw(ctx, rx_remote, redraw_exit_flag);
            });
            return Ok(Box::new(ui::Ui::new(config, exit_flag, tx, rx_ui)));
        }),
    );
}

fn redraw(
    ctx: egui::Context,
    mut rx: broadcast::Receiver<ClientCommand>,
    exit_flag: Arc<AtomicBool>,
) {
    loop {
        if exit_flag.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        while let Ok(_) = rx.try_recv() {
            ctx.request_repaint();
        }
    }
}
