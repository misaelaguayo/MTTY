use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use commands::Command;
use config::Config;
use eframe::egui::{self};
use tokio::sync::mpsc;

pub mod commands;
pub mod config;
pub mod fonts;
pub mod statemachine;
pub mod styles;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() {
    let config = config::Config::default();

    // Flag set when ui is closed and signals background threads to exit
    let exit_flag = Arc::new(AtomicBool::new(false));

    let term = term::Term::new(&config).unwrap();
    let read_fd = term.parent.try_clone().unwrap();
    let write_fd = term.parent.try_clone().unwrap();
    let (output_tx, output_rx) = mpsc::channel(10000);
    let (input_tx, input_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(10000);

    term::spawn_read_thread(read_fd.as_raw_fd(), exit_flag.clone(), output_tx);
    term::spawn_write_thread(write_fd, input_rx, exit_flag.clone());

    start_ui(&config, exit_flag, input_tx, output_rx);
}

fn start_ui(
    config: &Config,
    exit_flag: Arc<AtomicBool>,
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Command>,
) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([config.width, config.height]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "MTTY",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            fonts::configure_text_styles(&ctx, &config);
            // thread::spawn(|| {
            //     redraw(ctx);
            // });
            return Ok(Box::new(ui::Ui::new(config, exit_flag, tx, rx)));
        }),
    );
}

fn _redraw(ctx: egui::Context) {
    // This function was originally used because egui does not
    // update the screen if there are no events.
    // We needed to be able to update the screen when we got
    // Some data from the terminal.
    // This was a placeholder for that using a timer, but
    // it is not needed anymore.
    loop {
        thread::sleep(std::time::Duration::from_millis(10));
        ctx.request_repaint();
    }
}
