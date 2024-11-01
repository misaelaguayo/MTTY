use crossbeam::channel::{Receiver, Sender};
use dyn_clone::DynClone;
use log::{error, info};

use crate::{commands::read_command, term::Command};

pub trait Backend: DynClone {
    fn execute(&mut self);
}

#[derive(Clone)]
pub struct AsyncBackend {
    pub sender: Sender<Command>,
    pub receiver: Receiver<Command>,
}

impl Backend for AsyncBackend {
    fn execute(&mut self) {
        loop {
            let command = self.receiver.try_recv();
            if let Ok(command) = command {
                info!("Received command: {}", command.command);

                let iterator = match read_command(command.clone()) {
                    Ok(iterator) => iterator,
                    Err(e) => {
                        error!("Error reading command: {}", e);
                        return;
                    }
                };

                for output in iterator {
                    self.sender
                        .send(Command {
                            id: command.id,
                            command: command.command.clone(),
                            args: command.args.clone(),
                            response: vec![String::from_utf8(vec![output]).unwrap()],
                        })
                        .unwrap();
                }
            }
        }
    }
}

impl AsyncBackend {
    pub fn build(sender: Sender<Command>, receiver: Receiver<Command>) -> Self {
        Self { sender, receiver }
    }
}
