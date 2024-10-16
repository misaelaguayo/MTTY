use crossbeam::channel::{Receiver, Sender};
use dyn_clone::DynClone;

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
                println!("Received command: {}", command.command);
                let output = match read_command(command.clone()) {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output);
                        println!("Output: {}", stdout);
                        vec![stdout.to_string()]
                    }
                    Err(e) => {
                        println!("Error: {}", e.to_string());
                        vec![e.to_string()]
                    }
                };
                self.sender
                    .send(Command {
                        id: command.id,
                        command: command.command,
                        args: command.args,
                        response: output,
                    })
                    .unwrap();
            }
        }
    }
}

impl AsyncBackend {
    pub fn build(sender: Sender<Command>, receiver: Receiver<Command>) -> Self {
        Self { sender, receiver }
    }
}
