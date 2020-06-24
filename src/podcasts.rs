use crate::{
    file_system::{FilePermissions, FileSystem},
    web, Config, Errors,
};
use clap::{ArgMatches, Values};
use colored::*;
use csv;
use rss;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
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
            return self.add(&add_values);
        }

        if let Some(remove_values) = self.matches.values_of("remove") {
            return self.remove(&remove_values);
        }

        if self.matches.is_present("list") {
            return self.list();
        }

        Ok(())
    }

    /// Adds the passed podcasts values to the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory
    fn add(&self, add_values: &Values) -> Result<(), Errors> {
        let values = add_values.clone();

        let podcasts_list_file = FileSystem::open_podcasts_list(
            &self.config.app_directory,
            vec![FilePermissions::Read, FilePermissions::Append],
        )?;
        let mut reader = csv::Reader::from_reader(&podcasts_list_file);

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
            csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(podcasts_list_file)
        } else {
            csv::WriterBuilder::new()
                .has_headers(true)
                .from_writer(podcasts_list_file)
        };

        for podcast in podcasts {
            writer.serialize(podcast)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Remove the passed podcasts from the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory. does nothing if the passed values are not present in the file
    fn remove(&self, remove_values: &Values) -> Result<(), Errors> {
        let mut values = remove_values.clone();

        let podcasts_list_file =
            FileSystem::open_podcasts_list(&self.config.app_directory, vec![FilePermissions::Read])?;
        let mut reader = csv::Reader::from_reader(podcasts_list_file);

        // We overwrite the whole file with the remaining podcasts (minus the ones passed as args)
        let filtered_podcasts: Vec<Podcast> = reader
            .deserialize()
            .filter_map(|item: Result<Podcast, csv::Error>| item.ok())
            .filter(|podcast| !values.any(|value| value.trim() == podcast.rss_url))
            .collect();

        // Reopen file because truncation happens right after the opening of the file
        let podcasts_list_file =
            FileSystem::open_podcasts_list(&self.config.app_directory, vec![FilePermissions::WriteTruncate])?;

        let mut writer = csv::Writer::from_writer(podcasts_list_file);
        for podcast in filtered_podcasts {
            writer.serialize(podcast)?;
        }

        writer.flush()?;

        Ok(())
    }

    /// Lists the saved podcasts
    fn list(&self) -> Result<(), Errors> {
        let podcasts_list_file =
            FileSystem::open_podcasts_list(&self.config.app_directory, vec![FilePermissions::Read])?;
        let mut reader = csv::Reader::from_reader(&podcasts_list_file);

        for value in reader.deserialize() {
            let podcast: Podcast = value?;
            println!("{}", podcast);
        }

        Ok(())
    }
}
