pub struct State {
    commands: Vec<String>
}

pub trait Frontend {
    fn r#type(&self);
}

pub struct Terminal {
   pub frontend: Box<dyn Frontend>,
   pub backend: State
}


