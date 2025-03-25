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
    cursor: (u32, u32),
}

impl Ui {
    pub fn new(exit_flag: Arc<AtomicBool>, tx: Sender<Vec<u8>>, rx: Receiver<Command>) -> Self {
        Self {
            exit_flag,
            output: String::new(),
            input: String::new(),
            tx,
            rx,
            cursor: (0, 0),
        }
    }

    fn handle_event(&mut self, event: &egui::Event) {
        match event {
            egui::Event::Key {
                key: egui::Key::Enter,
                ..
            } => {
                self.input.push('\n');
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                ..
            } => {
                self.exit_flag
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            egui::Event::Key {
                key: egui::Key::Backspace,
                ..
            } => {
                self.input.pop();
            }
            egui::Event::Text(text) => {
                self.input.push_str(text);
            }
            _ => {}
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

        if !self.input.is_empty(){
            // self.output.push_str(&self.input);
            let _ = self.tx.try_send(self.input.as_bytes().to_vec());

            self.input.clear();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.input(|i| {
                i.raw.events.iter().for_each(|event| {
                    self.handle_event(event);
                });
            });

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(&self.output);
                ui.label(&self.input);
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
