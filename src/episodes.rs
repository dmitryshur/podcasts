use crate::{
    file_system::{FilePermissions, FileSystem},
    podcasts::Podcast,
    Config, Errors,
};
use clap::ArgMatches;
use csv;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

pub struct Episodes<'a> {
    matches: &'a ArgMatches,
    config: &'a Config,
}

impl<'a> Episodes<'a> {
    pub fn new(matches: &'a ArgMatches, config: &'a Config) -> Self {
        Self { matches, config }
    }

    pub fn run(&self) -> Result<(), Errors> {
        if let Some(matches) = self.matches.subcommand_matches("update") {
            let podcasts_list = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::Read],
            )
            .open()?;

            if let Some(ids) = matches.values_of("id") {
                let ids: HashSet<u64> = ids.flat_map(|id| id.parse::<u64>()).collect();
                let mut reader = csv::Reader::from_reader(&podcasts_list);
                let podcasts: Vec<Podcast> = reader
                    .deserialize()
                    .filter_map(|item: Result<Podcast, csv::Error>| item.ok())
                    .filter(|podcast| ids.contains(&podcast.id))
                    .collect();

                let mut files = HashMap::new();
                for podcast in podcasts.iter() {
                    let file = FileSystem::new(
                        &self.config.app_directory,
                        &podcast.id.to_string(),
                        vec![FilePermissions::Write],
                    )
                    .open();

                    if let Err(error) = file {
                        println!("Can't open file for podcast {}. {}", podcast.title, error);
                        continue;
                    }

                    files.insert(podcast.id, file.unwrap());
                }

                return self.update(&podcasts, files);
            }
        }

        match self.matches.subcommand_matches("list") {
            _ => {}
        }

        match self.matches.subcommand_matches("download") {
            _ => {}
        }

        match self.matches.subcommand_matches("remove") {
            _ => {}
        }

        match self.matches.subcommand_matches("archive") {
            _ => {}
        }

        Ok(())
    }

    // TODO fetch the rss feed. parse and save the needed data.
    // TODO hash the content and save the hash.
    pub fn update<T>(&self, podcasts: &Vec<Podcast>, writers: HashMap<u64, T>) -> Result<(), Errors>
    where
        T: Write,
    {
        println!("{:?}", podcasts);
        todo!()
    }

    pub fn list(&self) -> Result<(), Errors> {
        todo!()
    }

    pub fn download(&self) -> Result<(), Errors> {
        todo!()
    }

    pub fn remove(&self) -> Result<(), Errors> {
        todo!()
    }

    pub fn archive(&self) -> Result<(), Errors> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update() {
        todo!()
    }

    #[test]
    fn list() {
        todo!()
    }

    #[test]
    fn download() {
        todo!()
    }

    #[test]
    fn remove() {
        todo!()
    }

    #[test]
    fn archive() {
        todo!()
    }
}
