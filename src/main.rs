use std::{os::fd::{AsFd, AsRawFd}, thread};

use eframe::egui;
use term::{read_from_raw_fd, write_to_fd};
use tokio::sync::mpsc;

pub mod statemachine;
pub mod term;
pub mod ui;

#[tokio::main]
async fn main() {
    let term = term::Term::new().unwrap();
    let read_raw_fd = term.parent.try_clone().unwrap();
    let write_fd = term.parent.try_clone().unwrap();
    let (output_tx, output_rx) = mpsc::channel(10000);
    let (input_tx, mut input_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
        mpsc::channel(100);

    tokio::spawn(async move {
        let mut statemachine = vte::Parser::new();
        let mut performer = statemachine::StateMachine::new(output_tx);

        loop {
            if let Some(data) = read_from_raw_fd(read_raw_fd.as_raw_fd()) {
                statemachine.advance(&mut performer, &data);
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Some(data) = input_rx.recv().await {
                write_to_fd(write_fd.as_fd(), &data);
            }
        }
    });

    draw(input_tx, output_rx);
}

fn draw(tx: mpsc::Sender<Vec<u8>>, rx: mpsc::Receiver<Vec<u8>>) {
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
            return Ok(Box::new(ui::Ui::new(tx, rx)));
        }),
    );
}

fn redraw(ctx: egui::Context){
    loop {
        thread::sleep(std::time::Duration::from_millis(10));
        ctx.request_repaint();
    }
}
