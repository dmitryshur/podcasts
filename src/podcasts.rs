use crate::{
    file_system::{FilePermissions, FileSystem},
    web, Config, Errors,
};
use clap::{ArgMatches, Values};
use colored::*;
use csv;
use rss;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    fmt,
    hash::{Hash, Hasher},
    io::{Read, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Podcast {
    id: u64,
    url: String,
    rss_url: String,
    title: String,
}

impl fmt::Display for Podcast {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = format!("{:12}{}\n", "Title:".green(), self.title);
        str.push_str(&format!("{:12}{}\n", "Site URL:".green(), self.url));
        str.push_str(&format!("{:12}{}\n", "RSS URL:".green(), self.rss_url));
        str.push_str(&format!("{:12}{}\n", "ID:".green(), self.id));
        write!(f, "{}", str)
    }
}

#[derive(Debug)]
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
    pub fn run(&self) -> Result<(), Errors> {
        if let Some(add_values) = &self.matches.values_of("add") {
            let reader_file = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::Read],
            )
            .open()?;

            let writer_file = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::Read, FilePermissions::Append],
            )
            .open()?;

            return self.add(&add_values, reader_file, writer_file);
        }

        if let Some(remove_values) = self.matches.values_of("remove") {
            let mut reader_file = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::Read],
            )
            .open()?;

            // WriteTruncate mode erases file content, so we extract it here
            let mut contents = String::new();
            reader_file.read_to_string(&mut contents)?;

            let writer_file = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::WriteTruncate],
            )
            .open()?;

            return self.remove(&remove_values, contents.as_bytes(), writer_file);
        }

        if self.matches.is_present("list") {
            let reader_file = FileSystem::new(
                &self.config.app_directory,
                "podcast_list.csv",
                vec![FilePermissions::Read],
            )
            .open()?;
            let writer = std::io::stdout();
            let writer = writer.lock();

            return self.list(reader_file, writer);
        }

        Ok(())
    }

    /// Adds the passed podcasts values to the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory
    fn add<R, W>(&self, add_values: &Values, reader: R, writer: W) -> Result<(), Errors>
    where
        R: Read,
        W: Write,
    {
        let values = add_values.clone();
        let mut reader = csv::Reader::from_reader(reader);

        // Load previously saved URLs
        let saved_urls: HashSet<String> = reader
            .deserialize()
            .filter_map(|item: Result<Podcast, csv::Error>| item.map(|podcast| podcast.rss_url).ok())
            .collect();

        // Work only with new URLs
        let urls: Vec<&str> = values
            .map(|value| value.trim())
            .filter(|value| {
                return !saved_urls.contains(*value);
            })
            .collect();

        let mut hasher = DefaultHasher::new();
        let podcasts: Vec<Podcast> = web::Web::new()
            .get(&urls)
            .iter()
            .filter_map(|(url, response)| match response {
                Ok(res) => {
                    // Parse RSS feed
                    let rss_channel = rss::Channel::read_from(&res[..]);
                    if rss_channel.is_err() {
                        return None;
                    }
                    let rss_channel = rss_channel.unwrap();

                    // Get needed data from RSS feed and return new Podcast struct
                    let podcast_title = rss_channel.title().to_string();
                    let podcast_url = rss_channel.link().to_string();
                    let rss_url = url.to_string();
                    url.hash(&mut hasher);

                    Some(Podcast {
                        id: hasher.finish(),
                        url: podcast_url,
                        rss_url,
                        title: podcast_title,
                    })
                }
                Err(_err) => None,
            })
            .collect();

        // If some podcasts were previously saved, append with no headers
        let mut writer = if saved_urls.len() > 0 {
            csv::WriterBuilder::new().has_headers(false).from_writer(writer)
        } else {
            csv::WriterBuilder::new().has_headers(true).from_writer(writer)
        };

        for podcast in podcasts {
            writer.serialize(podcast)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Remove the passed podcasts from the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory. does nothing if the passed values are not present in the file
    fn remove<R, W>(&self, remove_values: &Values, reader: R, writer: W) -> Result<(), Errors>
    where
        R: Read,
        W: Write,
    {
        let values: Vec<&str> = remove_values.clone().collect();
        let mut reader = csv::Reader::from_reader(reader);

        // We overwrite the whole file with the remaining podcasts (minus the ones passed as args)
        let filtered_podcasts: Vec<Podcast> = reader
            .deserialize()
            .filter_map(|item: Result<Podcast, csv::Error>| item.ok())
            .filter(|podcast| values.iter().all(|value| *value != podcast.rss_url))
            .collect();

        let mut writer = csv::Writer::from_writer(writer);
        for podcast in filtered_podcasts {
            writer.serialize(podcast)?;
        }

        writer.flush()?;

        Ok(())
    }

    /// Lists the saved podcasts
    fn list<R, W>(&self, reader: R, mut writer: W) -> Result<(), Errors>
    where
        R: Read,
        W: Write,
    {
        let mut reader = csv::Reader::from_reader(reader);

        for value in reader.deserialize() {
            let podcast: Podcast = value?;
            writeln!(writer, "{}", podcast)?;
        }

        Ok(())
    }
}
