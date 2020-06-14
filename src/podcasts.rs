use crate::file_system;
use crate::Config;
use clap::{ArgMatches, Values};
use csv;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

pub struct Podcasts<'a> {
    matches: &'a ArgMatches,
    config: &'a Config,
}

impl<'a> Podcasts<'a> {
    /// Constructs a new Podcasts struct which is used to work with the sub command "podcasts"
    pub fn new(matches: &'a ArgMatches, config: &'a Config) -> Self {
        Self { matches, config }
    }

    /// Continues to match the rest of the passed arguments to the podcasts sub command
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

    /// Adds the passed podcasts values to the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory
    fn add(&self, _add_values: &Values) {
        let podcasts_list_file =
            file_system::FileSystem::open_podcasts_list(&self.config.app_directory);
        println!("{:?}", podcasts_list_file);
    }

    /// Remove the passed podcasts from the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory. does nothing if the passed values are not present in the file
    fn remove(&self, _remove_values: &Values) {
        unimplemented!();
    }

    /// Lists the saved podcasts
    fn list(&self) {
        unimplemented!();
    }
}
