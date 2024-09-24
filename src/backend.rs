use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{commands::read_command, term::State};

#[async_trait]
pub trait Backend {
    async fn execute(&mut self, command: String);
}

pub struct AsyncBackend {
    state: Arc<Mutex<State>>,
}

#[async_trait]
impl Backend for AsyncBackend {
    async fn execute(&mut self, command: String) {
        let mut state = self.state.lock().unwrap();
        let mut new_commands = state.commands.clone();
        new_commands.push(command.clone());
        let new_output = read_command(command);

        *state = State {
            commands: new_commands,
            last_output: new_output.unwrap(),
        };
    }
}

impl AsyncBackend {
    pub fn build(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}
