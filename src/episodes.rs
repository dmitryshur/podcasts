use crate::{
    file_system::{FilePermissions, FileSystem},
    podcasts::Podcast,
    web::Web,
    Config, Errors,
};
use clap::ArgMatches;
use colored::*;
use csv;
use rss;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::{Read, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Episode {
    guid: String,
    title: String,
    pub_date: String,
    link: String,
    podcast: String,
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = format!("{:14}{}\n", "Title:".green(), self.title);
        str.push_str(&format!("{:14}{}\n", "Release date:".green(), self.pub_date));
        str.push_str(&format!("{:14}{}\n", "ID:".green(), self.guid));
        str.push_str(&format!("{:14}{}\n", "Link:".green(), self.link));
        write!(f, "{}", str)
    }
}

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

        if let Some(matches) = self.matches.subcommand_matches("list") {
            match matches.values_of("id") {
                // Ids were passed as arguments to the list subcommand
                Some(ids) => {
                    let ids: HashSet<u64> = ids.flat_map(|id| id.parse::<u64>()).collect();
                    for id in ids {
                        let episodes_file =
                            FileSystem::new(&self.config.app_directory, &id.to_string(), vec![FilePermissions::Read])
                                .open();

                        // The file might not exist because it's created only after the update command
                        if let Ok(file) = episodes_file {
                            let mut reader = csv::Reader::from_reader(&file);
                            let episodes: Vec<Episode> = reader
                                .deserialize()
                                .filter_map(|item: Result<Episode, csv::Error>| item.ok())
                                .collect();
                            let podcast_title = episodes.get(0).ok_or(Errors::WrongID(id))?;
                            println!("{}:\n", podcast_title);
                            episodes.iter().for_each(|episode| {
                                println!("{}", episode);
                            })
                        }
                    }
                }
                // No Ids were passed. list all the episodes of all the saved podcasts
                None => {
                    let podcasts_list = FileSystem::new(
                        &self.config.app_directory,
                        "podcast_list.csv",
                        vec![FilePermissions::Read],
                    )
                    .open()?;
                    let mut reader = csv::Reader::from_reader(&podcasts_list);
                    let podcasts: Vec<Podcast> = reader
                        .deserialize()
                        .filter_map(|item: Result<Podcast, csv::Error>| item.ok())
                        .collect();

                    for podcast in podcasts.iter() {
                        let file = FileSystem::new(
                            &self.config.app_directory,
                            &podcast.id.to_string(),
                            vec![FilePermissions::Read],
                        )
                        .open();

                        // The file might not exist because it's created only after the update command
                        if let Ok(file) = file {
                            println!("{}:\n", podcast.title);
                            let mut reader = csv::Reader::from_reader(&file);
                            reader
                                .deserialize()
                                .filter_map(|item: Result<Episode, csv::Error>| item.ok())
                                .for_each(|item| {
                                    println!("{}", item);
                                });
                        }
                    }
                }
            }
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

    pub fn update<T>(&self, podcasts: &Vec<Podcast>, mut writers: HashMap<u64, T>) -> Result<(), Errors>
    where
        T: Write,
    {
        let urls_map: HashMap<&str, u64> = podcasts
            .iter()
            .map(|podcast| (podcast.rss_url.as_str(), podcast.id))
            .collect();

        let urls: Vec<&str> = podcasts.iter().map(|podcast| podcast.rss_url.as_str()).collect();

        for result in Web::new().get(&urls) {
            if let Ok(response) = result.1 {
                let rss_channel = rss::Channel::read_from(&response[..]);
                if rss_channel.is_err() {
                    continue;
                }
                let rss_channel = rss_channel.unwrap();

                let podcast_title = rss_channel.title();
                // We collect guid, pub_date, title, link from the rss feed for each item
                let items: Vec<Episode> = rss_channel
                    .items()
                    .iter()
                    .filter_map(|item| {
                        let guid = item.guid();
                        let pub_date = item.pub_date();
                        let title = item.title();
                        let link = item.link();

                        match (guid, pub_date, title, link) {
                            (Some(guid), Some(pub_date), Some(title), Some(link)) => Some(Episode {
                                guid: guid.value().to_string(),
                                pub_date: pub_date.to_string(),
                                title: title.to_string(),
                                link: link.to_string(),
                                podcast: podcast_title.to_string(),
                            }),
                            _ => None,
                        }
                    })
                    .collect();

                let id = urls_map.get(result.0).ok_or(Errors::RSS)?;
                let mut writer = writers.get_mut(id).ok_or(Errors::RSS)?;
                let mut csv_writer = csv::WriterBuilder::new().has_headers(true).from_writer(writer);

                for item in items {
                    csv_writer.serialize(item)?;
                }

                csv_writer.flush()?;
            }
        }

        Ok(())
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
