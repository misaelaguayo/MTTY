use crate::backend::{AsyncBackend, Backend};
use crate::config::Config;
use crate::frontend::{Frontend, Sdl2TerminalFrontend};

pub struct Terminal {
    pub frontend: Box<dyn Frontend>,
    pub backend: Box<dyn Backend>,
}

impl Terminal {
    pub fn build(config: Config) -> Terminal {
        Terminal {
            frontend: Box::new(Sdl2TerminalFrontend::build(config)),
            backend: Box::new(AsyncBackend::new()),
        }
    }

    pub fn run(&mut self) {
        self.frontend.poll_event();
    }
}
