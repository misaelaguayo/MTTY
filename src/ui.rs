use std::sync::{atomic::AtomicBool, Arc};

use eframe::egui::{self, Rect, Widget};
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

    fn handle_event(&mut self, event: &egui::Event) {
        match event {
            egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                ..
            } => {
                self.input.push('\n');
            }
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                ..
            } => {
                // ASCII code for backspace
                self.tx.try_send(vec![8]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                self.tx.try_send(vec![27]).unwrap();
            }
            egui::Event::Key {
                key: egui::Key::Space,
                pressed: true,
                ..
            } => {
                self.input.push(' ');
            }
            egui::Event::Key {
                key: egui::Key::Minus,
                pressed: true,
                ..
            } => {
                self.input.push('-');
            }
            egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } => {
                match modifiers {
                    egui::Modifiers { shift: true, .. } => {
                        self.input.push_str(&key.name());
                    }
                    egui::Modifiers { ctrl: true, .. } => {
                        match key.name() {
                            "C" => {
                                self.tx.try_send(vec![3]).unwrap();
                            }
                            "D" => {
                                self.tx.try_send(vec![4]).unwrap();
                            }
                            "L" => {
                                self.tx.try_send(vec![12]).unwrap();
                            }
                            "U" => {
                                self.tx.try_send(vec![21]).unwrap();
                            }
                            "W" => {
                                self.tx.try_send(vec![23]).unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        self.input.push_str(&key.name().to_lowercase());
                    }
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for Ui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(data) = self.rx.try_recv().ok() {
            match data {
                Command::Backspace => {
                    self.output.pop();
                }
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

        if !self.input.is_empty() {
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
                ui.monospace(&self.output);
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.exit_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
