use std::sync::{Arc, Mutex};
use std::thread;

use crate::backend::{AsyncBackend, Backend};
use crate::config::Config;
use crate::frontend::{Frontend, Sdl2TerminalFrontend};

pub struct State {
    pub commands: Vec<String>,
    pub last_output: Vec<String>,
}

pub struct Terminal {
    pub frontend: Box<dyn Frontend>,
    pub backend: Box<dyn Backend>,
    pub state: Arc<Mutex<State>>,
}

impl Terminal {
    pub fn build(config: Config) -> Terminal {
        let state = Arc::new(Mutex::new(State {
            commands: Vec::new(),
            last_output: Vec::new(),
        }));

        Terminal {
            frontend: Box::new(Sdl2TerminalFrontend::build(config, state.clone())),
            backend: Box::new(AsyncBackend::build(state.clone())),
            state,
        }
    }

    pub fn run(&mut self) {
        self.frontend.poll_event();
    }
}
