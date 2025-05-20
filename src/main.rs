use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use commands::Command;
use config::Config;
use eframe::egui::{self};
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

    start_ui(&config, exit_flag, input_tx, output_rx_ui);

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: Arc<AtomicBool>,
    tx: broadcast::Sender<Vec<u8>>,
    rx_ui: broadcast::Receiver<Command>,
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

fn redraw(ctx: egui::Context, mut rx: broadcast::Receiver<Command>, exit_flag: Arc<AtomicBool>) {
    loop {
        if exit_flag.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        while let Ok(_) = rx.try_recv() {
            ctx.request_repaint();
        }
    }
}
