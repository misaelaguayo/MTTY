use eframe::egui;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Ui {
    output: String,
    input: String,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl Ui {
    pub fn new(tx: Sender<Vec<u8>>,rx: Receiver<Vec<u8>>) -> Self {
        Self {
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
            let data = String::from_utf8(data).unwrap();
            self.output.push_str(&data);
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
}
