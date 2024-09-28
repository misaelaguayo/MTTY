use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::backend::{AsyncBackend, Backend};
use crate::config::Config;
use crate::frontend::{Frontend, Sdl2TerminalFrontend};

pub struct Terminal {
    pub frontend: Box<dyn Frontend>,
    pub backend: Box<dyn Backend + Send>,
}

impl Terminal {
    pub fn build(config: Config) -> Terminal {
        let (backend_sender, backend_receiver): (Sender<Vec<String>>, Receiver<Vec<String>>) =
            unbounded();
        let (frontend_sender, frontend_receiver): (Sender<Vec<String>>, Receiver<Vec<String>>) =
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
