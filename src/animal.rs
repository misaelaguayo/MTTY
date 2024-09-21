struct Sheep {
    speech: String,
}
struct Cow {
    speech: String,
}

impl Sheep {
    fn new() -> Sheep {
        Sheep {
            speech: String::from("baaaah!"),
        }
    }
}

impl Cow {
    fn new() -> Cow {
        Cow {
            speech: String::from("mooo!"),
        }
    }
}

pub trait Animal {
    fn noise(&self) -> &'static str;
    fn speech(&mut self, words: String);
}

impl Animal for Sheep {
    fn noise(&self) -> &'static str {
        "baaaah!"
    }
    fn speech(&mut self, words: String) {
        self.speech = words;
    }
}

impl Animal for Cow {
    fn noise(&self) -> &'static str {
        "mooo!"
    }
    fn speech(&mut self, words: String) {
        self.speech = words;
    }
}

pub fn random_animal(random_number: f64) -> Box<dyn Animal> {
    if random_number < 0.5 {
        Box::new(Sheep::new())
    } else {
        Box::new(Sheep::new())
    }
}
