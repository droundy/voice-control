use std::fmt::Debug;

pub mod keys;

pub struct Action {
    f: Box<dyn Fn() + Sync + Send>,
    name: String,
}

impl Action {
    pub fn run(&self) {
        (self.f)()
    }

    pub fn new(name: String, f: impl 'static + Fn() + Sync + Send) -> Self {
        Action {
            f: Box::new(f),
            name,
        }
    }

    pub fn only_log(input: &str) -> Self {
        let input = input.to_string();
        Action {
            name: format!("log {:?}", input),
            f: Box::new(move || println!("{}", input)),
        }
    }
}

impl Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}
