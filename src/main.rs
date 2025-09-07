use std::sync::{atomic::AtomicBool, Arc};

use commands::ClientCommand;
use config::Config;
use eframe::egui::{self, IconData};
use std::io::Write;
use tokio::sync::broadcast;

use crate::{commands::ServerCommand, ui::Ui};

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
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} - [{}] {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.level(),
                record.args()
            )
        })
        .init();

    let app = app::App::new(Config::default(), Arc::new(AtomicBool::new(false)));

    start_ui(
        &app.config,
        &app.is_running,
        &app.server_channel.input_transmitter,
        &app.client_channel.output_receiver,
    );

    Ok(())
}

fn start_ui(
    config: &Config,
    exit_flag: &Arc<AtomicBool>,
    tx: &broadcast::Sender<ServerCommand>,
    ui_update_receiver: &broadcast::Receiver<ClientCommand>,
) {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_icon(IconData::default())
            .with_inner_size([config.width, config.height]),
        ..Default::default()
    };

    let redraw_update_receiver = ui_update_receiver.resubscribe();
    let redraw_exit_flag = exit_flag.clone();
    let ui = Ui::new(config, exit_flag.clone(), tx.clone(), ui_update_receiver.resubscribe());

    let _ = eframe::run_native(
        "MTTY",
        options,
        Box::new(|cc| {
            let ctx = cc.egui_ctx.clone();
            fonts::configure_text_styles(&ctx, &config);
            tokio::spawn(async move {
                redraw(ctx, redraw_update_receiver, redraw_exit_flag);
            });

            return Ok(Box::new(ui));
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
