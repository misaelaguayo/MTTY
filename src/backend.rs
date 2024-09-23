pub trait Backend {
    fn execute(&mut self, command: String);
}

pub struct AsyncBackend {
    commands: State,
}

impl Backend for AsyncBackend {
    fn execute(&mut self, command: String) {
        self.commands.commands.push(command);
    }
}

impl AsyncBackend {
    pub fn new() -> Self {
        Self {
            commands: State {
                commands: Vec::new(),
            },
        }
    }
}

pub struct State {
    commands: Vec<String>,
}
