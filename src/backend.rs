use crossbeam::channel::{Receiver, Sender};
use dyn_clone::DynClone;

use crate::commands::read_command;

pub trait Backend: DynClone {
    fn execute(&mut self);
}

#[derive(Clone)]
pub struct AsyncBackend {
    pub sender: Sender<Vec<String>>,
    pub receiver: Receiver<Vec<String>>,
}

impl Backend for AsyncBackend {
    fn execute(&mut self) {
        loop {
            let commands = self.receiver.try_recv();
            if let Ok(commands) = commands {
                for command in commands {
                    println!("Received command: {}", command);
                    let output = read_command(command).unwrap();
                    self.sender.send(output).unwrap();
                }
            }
        }
    }
}

impl AsyncBackend {
    pub fn build(sender: Sender<Vec<String>>, receiver: Receiver<Vec<String>>) -> Self {
        Self { sender, receiver }
    }
}
