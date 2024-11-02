use crossbeam::channel::{unbounded, Receiver, Sender};
use uuid::Uuid;

use crate::backend::{AsyncBackend, Backend};
use crate::config::Config;
use crate::frontend::Frontend;
use crate::sdl2frontend::Sdl2TerminalFrontend;

pub struct Terminal {
    pub frontend: Box<dyn Frontend>,
    pub backend: Box<dyn Backend + Send>,
}

#[derive(Clone)]
pub struct Command {
    pub id: Uuid,
    pub command: String,
    pub args: Vec<String>,
    pub response: Vec<String>,
}

pub struct CommandOutputIterator {
    pub output: Vec<u8>,
}

impl Terminal {
    pub fn build(config: Config) -> Terminal {
        let (backend_sender, backend_receiver): (Sender<Command>, Receiver<Command>) = unbounded();
        let (frontend_sender, frontend_receiver): (Sender<Command>, Receiver<Command>) =
            unbounded();

        let backend = Box::new(AsyncBackend::build(backend_sender, frontend_receiver));
        let frontend = Box::new(Sdl2TerminalFrontend::build(
            config,
            frontend_sender,
            backend_receiver,
        ));

        Terminal { frontend, backend }
    }

    pub fn run(&mut self) {
        self.frontend.poll_event();
    }
}
