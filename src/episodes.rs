use crate::{
    file_system::{FilePermissions, FileSystem},
    podcasts::Podcast,
    web::Web,
    Config, Errors,
};
use bytes::{Buf, BufMut, Bytes};
use clap::{ArgMatches, Values};
use colored::*;
use csv;
use rss;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::{self, Read, Write},
    time,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Episode {
    guid: String,
    title: String,
    pub_date: String,
    link: String,
    podcast: String,
    podcast_id: u64,
}

impl fmt::Display for Episode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = format!("{:14}{}\n", "Title:".green(), self.title);
        str.push_str(&format!("{:14}{}\n", "Release date:".green(), self.pub_date));
        str.push_str(&format!("{:14}{}\n", "ID:".green(), self.guid));
        str.push_str(&format!("{:14}{}\n", "Link:".green(), self.link));
        str.push_str(&format!("{:14}{}\n", "Podcast:".green(), self.podcast));
        str.push_str(&format!("{:14}{}\n", "Podcast ID:".green(), self.podcast_id));
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
                    let files: Vec<(u64, File)> = ids
                        .flat_map(|id| {
                            let file =
                                FileSystem::new(&self.config.app_directory, id, vec![FilePermissions::Read]).open();
                            let file_id = id.parse::<u64>();
                            if file.is_err() || file_id.is_err() {
                                return None;
                            }

                            Some((file_id.unwrap(), file.unwrap()))
                        })
                        .collect();

                    for file in files {
                        let writer = std::io::stdout();
                        let writer = writer.lock();

                        if let Err(error) = self.list(file.1, writer) {
                            return Err(error);
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

                    // The files with the same as id as the the passed id arguments
                    let files: Vec<(u64, File)> = reader
                        .deserialize()
                        .filter_map(|item: Result<Podcast, csv::Error>| {
                            if item.is_err() {
                                return None;
                            }
                            let podcast = item.unwrap();
                            let file = FileSystem::new(
                                &self.config.app_directory,
                                &podcast.id.to_string(),
                                vec![FilePermissions::Read],
                            )
                            .open();
                            if file.is_err() {
                                return None;
                            }
                            Some((podcast.id, file.unwrap()))
                        })
                        .collect();

                    for file in files {
                        let writer = std::io::stdout();
                        let writer = writer.lock();

                        return self.list(file.1, writer);
                    }
                }
            }
        }

        if let Some(matches) = self.matches.subcommand_matches("download") {
            // Always present because it's a required argument
            let podcast_id = matches.value_of("id").unwrap();
            let episodes_file =
                FileSystem::new(&self.config.app_directory, podcast_id, vec![FilePermissions::Read]).open();

            if episodes_file.is_err() {
                return Err(Errors::WrongID(podcast_id.to_string()));
            }

            let episodes_file = episodes_file.unwrap();
            match matches.values_of("episode-id") {
                Some(ids) => {
                    let files_data = self.download(&ids, episodes_file)?;
                    for (file_name, content) in files_data {
                        let mut file = FileSystem::new(
                            &self.config.download_directory,
                            &file_name,
                            vec![FilePermissions::Write],
                        )
                        .open()?;
                        file.write_all(content.bytes())?;
                    }
                }
                // --list or --count arguments may be present
                None => {
                    let list_present = matches.is_present("list");
                    let count = matches.value_of("count");

                    match (list_present, count) {
                        // List all downloaded episodes for the podcast
                        (true, None) => {
                            let dir_files =
                                fs::read_dir(&self.config.download_directory).map_err(|error| Errors::IO(error))?;

                            let mut downloaded_episodes = Vec::new();
                            for dir_entry in dir_files {
                                let path = dir_entry?.path();
                                let entry = path
                                    .file_name()
                                    .ok_or(Errors::IO(io::Error::new(
                                        io::ErrorKind::Other,
                                        "Couldn't get file name",
                                    )))?
                                    .to_str();
                                if let Some(entry) = entry {
                                    downloaded_episodes.push(entry.to_string());
                                }
                            }
                            let writer = std::io::stdout();
                            let writer = writer.lock();
                            self.list_downloaded(episodes_file, downloaded_episodes, writer);
                        }
                        // List only N amount of episodes for the podcast
                        (true, Some(count)) => {
                            // TODO save as above but with count. refactor needed above
                        }
                        // Download last N amount of episodes for the podcast
                        (false, Some(count)) => {
                            // TODO get N latest episodes from episodes file. pass the ids to self.download
                        }
                        // Download all the existing episodes for the podcast
                        (false, None) => {
                            // TODO all the ids of the episodes from the episodes file. pass to self.download
                        }
                    }
                }
            }
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

        for (url, bytes) in Web::new(time::Duration::from_secs(10)).get(&urls) {
            let bytes = bytes?;
            let rss_channel = rss::Channel::read_from(&bytes[..]);
            if rss_channel.is_err() {
                continue;
            }
            let rss_channel = rss_channel.unwrap();

            let podcast_title = rss_channel.title();
            let podcast_id = urls_map.get(url).ok_or(Errors::RSS)?;
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
                            podcast_id: *podcast_id,
                        }),
                        _ => None,
                    }
                })
                .collect();

            let writer = writers.get_mut(podcast_id).ok_or(Errors::RSS)?;
            let mut csv_writer = csv::WriterBuilder::new().has_headers(true).from_writer(writer);

            for item in items {
                csv_writer.serialize(item)?;
            }

            csv_writer.flush()?;
        }

        Ok(())
    }

    pub fn list<R, W>(&self, reader: R, mut writer: W) -> Result<(), Errors>
    where
        R: Read,
        W: Write,
    {
        let mut csv_reader = csv::Reader::from_reader(reader);
        let episodes: Vec<Episode> = csv_reader
            .deserialize()
            .filter_map(|item: Result<Episode, csv::Error>| item.ok())
            .collect();
        for episode in episodes {
            writeln!(writer, "{}", episode)?;
        }

        Ok(())
    }

    pub fn download<R>(&self, ids: &Values, reader: R) -> Result<Vec<(String, Bytes)>, Errors>
    where
        R: Read,
    {
        let mut csv_reader = csv::Reader::from_reader(reader);
        let mut episode_ids: Vec<&str> = ids.clone().collect();
        let episodes: HashMap<String, Episode> = csv_reader
            .deserialize()
            .filter_map(|item: Result<Episode, csv::Error>| item.ok())
            .filter(|episode| episode_ids.iter().any(|id| *id == episode.guid))
            .map(|episode| (episode.link.clone(), episode))
            .collect();
        let episode_urls: Vec<&str> = episodes.keys().map(|key| key.as_str()).collect();

        let mut files_data = Vec::new();
        for (url, bytes) in Web::new(time::Duration::from_secs(0)).get(&episode_urls) {
            let bytes = bytes?;
            let episode = episodes.get(url).unwrap();
            let file_name = format!("{}_{}.mp3", episode.podcast, episode.title);
            files_data.push((file_name, bytes));
        }

        Ok(files_data)
    }

    pub fn remove(&self) -> Result<(), Errors> {
        todo!()
    }

    pub fn archive(&self) -> Result<(), Errors> {
        todo!()
    }

    fn list_downloaded<R, W>(&self, episodes: R, downloaded_episodes: Vec<String>, mut writer: W) -> Result<(), Errors>
    where
        R: Read,
        W: Write,
    {
        let mut csv_reader = csv::Reader::from_reader(episodes);
        let episodes: Vec<Episode> = csv_reader
            .deserialize()
            .filter_map(|item: Result<Episode, csv::Error>| item.ok())
            .filter(|episode| {
                let file_name = format!("{}_{}.mp3", episode.podcast, episode.title);
                downloaded_episodes.contains(&file_name)
            })
            .collect();

        for episode in episodes {
            writeln!(writer, "{}", episode)?;
        }

        Ok(())
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
