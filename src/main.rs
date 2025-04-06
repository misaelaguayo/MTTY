use std::{
    os::fd::AsRawFd,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use commands::Command;
use config::Config;
use eframe::egui::{self, FontFamily, FontId, TextStyle};
use tokio::sync::broadcast;

pub mod commands;
pub mod config;
pub mod statemachine;
pub mod styles;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() {
    let config = config::Config::default();
    let exit_flag = Arc::new(AtomicBool::new(false));

    let term = term::Term::new(&config).unwrap();
    let read_fd = term.parent.try_clone().unwrap();
    let write_fd = term.parent.try_clone().unwrap();
    let (output_tx, output_rx) = broadcast::channel(10000);
    let (input_tx, input_rx): (broadcast::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>) =
        broadcast::channel(100);

    term::spawn_read_thread(read_fd.as_raw_fd(), exit_flag.clone(), output_tx);
    term::spawn_write_thread(write_fd, input_rx, exit_flag.clone());

    start_ui(&config, exit_flag, input_tx, output_rx);
}

fn start_ui(
    config: &Config,
    exit_flag: Arc<AtomicBool>,
    tx: broadcast::Sender<Vec<u8>>,
    rx: broadcast::Receiver<Command>,
) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([config.width, config.height]),
        ..Default::default()
    };

    let rx2 = rx.resubscribe();
    let _ = eframe::run_native(
        "MTTY",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            configure_text_styles(&ctx, &config);
            thread::spawn(|| {
                redraw(ctx, rx);
            });
            return Ok(Box::new(ui::Ui::new(config, exit_flag, tx, rx2)));
        }),
    );
}

fn configure_text_styles(ctx: &egui::Context, config: &Config) {
    use FontFamily::Proportional;
    use TextStyle::*;

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (Heading, FontId::new(30.0, Proportional)),
        (Body, FontId::new(18.0, Proportional)),
        (Monospace, FontId::new(config.font_size, Proportional)),
        (Button, FontId::new(14.0, Proportional)),
        (Small, FontId::new(10.0, Proportional)),
    ]
    .into();
    ctx.set_style(style);
}

fn redraw(ctx: egui::Context, rx: broadcast::Receiver<Command>) {
    loop {
        if !rx.is_empty() {
            ctx.request_repaint();
        }
    }
}
