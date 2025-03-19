use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::commands::Command;

pub struct Ui {
    exit_flag: Arc<AtomicBool>,
    output: String,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Command>,
}

impl Ui {
    pub fn new(exit_flag: Arc<AtomicBool>, tx: Sender<Vec<u8>>, rx: Receiver<Command>) -> Self {
        Self {
            exit_flag,
            output: String::new(),
            input: String::new(),
            tx,
            rx,
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(data) = self.rx.try_recv().ok() {
            match data {
                Command::Print(c) => {
                    self.output.push(c);
                }
                Command::NewLine => {
                    self.output.push('\n');
                }
                Command::CarriageReturn => {
                    self.output.push('\r');
                }
                Command::ClearScreen => {
                    self.output.clear();
                }
                _ => {}
            }
        }

        if !self.input.is_empty() && self.input.ends_with('\n') {
            self.output.push_str(&self.input);
            let _ = self.tx.try_send(self.input.as_bytes().to_vec());

            self.input.clear();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MTTY");

            ui.label(&self.output);
            ui.text_edit_multiline(&mut self.input);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
