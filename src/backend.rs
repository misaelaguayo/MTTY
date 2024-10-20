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

                let iterator = match read_command(command.clone()) {
                    Ok(iterator) => iterator,
                    Err(e) => {
                        println!("Error: {}", e.to_string());
                        return;
                    }
                };

                for output in iterator {
                    print!("{}", output as char);
                    self.sender
                        .send(Command {
                            id: command.id,
                            command: command.command.clone(),
                            args: command.args.clone(),
                            response: vec![(output as char).to_string()],
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
