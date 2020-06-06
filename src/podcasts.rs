use clap::{ArgMatches, Values};

pub struct Podcasts<'a> {
    matches: &'a ArgMatches,
}

impl<'a> Podcasts<'a> {
    pub fn new(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    pub fn run(&self) {
        if let Some(add_values) = &self.matches.values_of("add") {
            self.add(&add_values);
        }

        if let Some(remove_values) = self.matches.values_of("remove") {
            self.remove(&remove_values);
        }

        if self.matches.is_present("list") {
            self.list();
        }
    }

    fn add(&self, _add_values: &Values) {
        unimplemented!();
    }

    fn remove(&self, _remove_values: &Values) {
        unimplemented!();
    }

    fn list(&self) {
        unimplemented!();
    }
}
