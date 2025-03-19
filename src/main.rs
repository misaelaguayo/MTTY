use std::{
    os::fd::{AsFd, AsRawFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use commands::Command;
use eframe::egui;
use term::write_to_fd;
use tokio::sync::mpsc;

pub mod commands;
pub mod statemachine;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() {
    let exit_flag = Arc::new(AtomicBool::new(false));
    let terminal_read_exit_flag = exit_flag.clone();
    let terminal_write_exit_flag = exit_flag.clone();

    let term = term::Term::new().unwrap();
    let read_raw_fd = term.parent.try_clone().unwrap();
    let write_fd = term.parent.try_clone().unwrap();
    let (output_tx, output_rx) = mpsc::channel(10000);
    let (input_tx, mut input_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(100);

    term::spawn_read_thread(read_raw_fd.as_raw_fd(), terminal_read_exit_flag, output_tx);

    tokio::spawn(async move {
        loop {
            if let Some(data) = input_rx.recv().await {
                write_to_fd(write_fd.as_fd(), &data);
            }

            if terminal_write_exit_flag.load(Ordering::Relaxed) {
                break;
            }
        }
    });

    draw(exit_flag, input_tx, output_rx);
}

fn draw(exit_flag: Arc<AtomicBool>, tx: mpsc::Sender<Vec<u8>>, rx: mpsc::Receiver<Command>) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "MTTY",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            thread::spawn(|| {
                redraw(ctx);
            });
            return Ok(Box::new(ui::Ui::new(exit_flag, tx, rx)));
        }),
    );
}

fn redraw(ctx: egui::Context) {
    loop {
        thread::sleep(std::time::Duration::from_millis(10));
        ctx.request_repaint();
    }
}
