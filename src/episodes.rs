use crate::{
    file_system::{FilePermissions, FileSystem},
    podcasts::Podcast,
    web::Web,
    Config, Errors,
};
use bytes::{Buf, Bytes};
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

                return self.update(&podcasts, &mut files);
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
                    let files_data = self.download(Some(&ids), episodes_file, None)?;
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
                    let count = if count.is_none() {
                        None
                    } else {
                        Some(count.unwrap().parse::<usize>()?)
                    };

                    match list_present {
                        // List downloaded episodes for the podcast. use count to indicate how many episodes
                        // to list
                        true => {
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
                            return self.list_downloaded(episodes_file, downloaded_episodes, writer, count);
                        }
                        false => {
                            let files_data = self.download(None, episodes_file, count)?;
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
                    }
                }
            }
        }

        Ok(())
    }

    pub fn update<T>(&self, podcasts: &Vec<Podcast>, writers: &mut HashMap<u64, T>) -> Result<(), Errors>
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
                        (Some(guid), Some(pub_date), Some(title), link) => Some(Episode {
                            guid: guid.value().to_string(),
                            pub_date: pub_date.to_string(),
                            title: title.to_string(),
                            link: link.unwrap_or("-").to_string(),
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
        for episode in episodes.iter().rev() {
            writeln!(writer, "{}", episode)?;
        }

        Ok(())
    }

    pub fn download<R>(
        &self,
        ids: Option<&Values>,
        reader: R,
        count: Option<usize>,
    ) -> Result<Vec<(String, Bytes)>, Errors>
    where
        R: Read,
    {
        let mut csv_reader = csv::Reader::from_reader(reader);
        let episode_ids: Option<Vec<&str>> = if ids.is_none() {
            None
        } else {
            Some(ids.unwrap().clone().collect())
        };

        let episodes: Vec<Episode> = csv_reader
            .deserialize()
            .filter_map(|item: Result<Episode, csv::Error>| item.ok())
            .filter(|episode| {
                // Download all the episodes if no ids were provided
                if episode_ids.is_none() {
                    return true;
                }

                episode_ids.as_ref().unwrap().iter().any(|id| *id == episode.guid)
            })
            .collect();
        let episodes_count = episodes.len();

        // Take count amount of episodes if needed
        let episodes_map: HashMap<String, Episode> = episodes
            .into_iter()
            .take(count.unwrap_or(episodes_count))
            .map(|episode| (episode.link.clone(), episode))
            .collect();
        let episode_urls: Vec<&str> = episodes_map.keys().map(|key| key.as_str()).collect();

        let mut files_data = Vec::new();
        for (url, bytes) in Web::new(time::Duration::from_secs(0)).get(&episode_urls) {
            let bytes = bytes?;
            let episode = episodes_map.get(url).unwrap();
            let file_name = format!("{}_{}.mp3", episode.podcast, episode.title);
            files_data.push((file_name, bytes));
        }

        Ok(files_data)
    }

    fn list_downloaded<R, W>(
        &self,
        episodes: R,
        downloaded_episodes: Vec<String>,
        mut writer: W,
        count: Option<usize>,
    ) -> Result<(), Errors>
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

        for (index, episode) in episodes.iter().rev().enumerate() {
            if let Some(count) = count {
                if index < count {
                    continue;
                }
            }

            writeln!(writer, "{}", episode)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Application, ApplicationBuilder};
    use clap::{App, Arg};
    use std::path::PathBuf;
    use std::str::from_utf8;

    fn create_config() -> Config {
        let app_directory = "/Users/dmitryshur/.podcasts";
        let download_directory = "/Users/dmitryshur/.podcasts/downloads";

        Config {
            app_directory: PathBuf::from(app_directory),
            download_directory: PathBuf::from(download_directory),
        }
    }

    fn create_app() -> Application {
        let config = create_config();
        ApplicationBuilder::new(config).episodes_subcommand().build()
    }

    #[test]
    fn update() {
        let app = create_app();
        let config = create_config();
        let args = app
            .app
            .get_matches_from(vec!["pcasts", "episodes", "update", "--id", "15913066141282366353"]);
        let episodes_matches = args.subcommand_matches("episodes").expect("No episodes matches");
        let episodes = Episodes::new(&episodes_matches, &config);
        let podcasts = vec![Podcast {
            id: 15913066141282366353,
            url: "https://syntax.fm".to_string(),
            rss_url: "https://feed.syntax.fm/rss".to_string(),
            title: "Syntax - Tasty Web Development Treats".to_string(),
        }];
        let mut syntax_expected_output = String::new();
        let mut file = File::open("src/test_files/syntax.csv").expect("Can't open syntax.csv");
        file.read_to_string(&mut syntax_expected_output)
            .expect("Can't write syntax.csv");

        let mut writers = HashMap::new();
        writers.insert(15913066141282366353, Vec::new());
        episodes.update(&podcasts, &mut writers);

        let syntax_output_string = from_utf8(writers.get(&15913066141282366353).unwrap()).unwrap();

        assert_eq!(syntax_output_string.trim(), syntax_expected_output.trim());
    }

    #[test]
    fn list_episodes() {
        let app = create_app();
        let config = create_config();
        let args = app.app.get_matches_from(vec!["pcasts", "episodes", "list"]);
        let episodes_matches = args.subcommand_matches("episodes").expect("No episodes matches");
        let episodes = Episodes::new(&episodes_matches, &config);

        let input = r###"guid,title,pub_date,link,podcast,podcast_id
272eca72-476b-4633-864c-a9fffa3f5976,Potluck - Beating Procrastination × Rollup vs Webpack × Leadership × Code Planning × Styled Components × More!,"Wed, 22 Jul 2020 13:00:00 +0000",https://traffic.libsyn.com/secure/syntax/Syntax268.mp3,Syntax - Tasty Web Development Treats,15913066141282366353"###;
        let input = input.as_bytes();
        let episode = Episode {
            guid: "272eca72-476b-4633-864c-a9fffa3f5976".to_string(),
            title: "Potluck - Beating Procrastination × Rollup vs Webpack × Leadership × Code Planning × Styled Components × More!".to_string(),
            pub_date: "Wed, 22 Jul 2020 13:00:00 +0000".to_string(),
            link: "https://traffic.libsyn.com/secure/syntax/Syntax268.mp3".to_string(),
            podcast: "Syntax - Tasty Web Development Treats".to_string(),
            podcast_id: 15913066141282366353
        };
        let expected_output = episode.to_string();
        let mut output = Vec::new();
        episodes.list(input, &mut output).expect("Can't list episodes");
        assert_eq!(from_utf8(&output).unwrap().trim(), expected_output.trim());
    }

    #[test]
    fn download() {
        let app = create_app();
        let config = create_config();
        let args = app
            .app
            .get_matches_from(vec!["pcasts", "episodes", "download", "--id", "15913066141282366353"]);
        let episodes_matches = args.subcommand_matches("episodes").expect("No episodes matches");
        let episode_id = episodes_matches.values_of("episode-id");
        let episodes = Episodes::new(&episodes_matches, &config);

        let input = r###"guid,title,pub_date,link,podcast,podcast_id
272eca72-476b-4633-864c-a9fffa3f5976,Potluck - Beating Procrastination × Rollup vs Webpack × Leadership × Code Planning × Styled Components × More!,"Wed, 22 Jul 2020 13:00:00 +0000",https://traffic.libsyn.com/secure/syntax/Syntax268.mp3,Syntax - Tasty Web Development Treats,15913066141282366353"###;
        let input = input.as_bytes();
        let expected_output = vec![(format!("{}_{}.mp3", "Syntax - Tasty Web Development Treats", "Potluck - Beating Procrastination × Rollup vs Webpack × Leadership × Code Planning × Styled Components × More!"), Bytes::from("Syntax episode"))];
        let output = episodes
            .download(episode_id.as_ref(), input, None)
            .expect("Can't download episodes");

        assert_eq!(output, expected_output);
    }
}
