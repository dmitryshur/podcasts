use crate::{file_system, web, Config, Errors};
use clap::{ArgMatches, Values};
use csv;
use rss;
use serde::Serialize;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Debug, Serialize)]
struct Podcast {
    id: u64,
    url: String,
    rss_url: String,
    title: String,
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
        let urls: Vec<&str> = values.map(|value| value).collect();
        let mut hasher = DefaultHasher::new();
        let podcasts: Vec<Option<Podcast>> = web::Web::new()
            .get(&urls)
            .iter()
            .map(|(url, response)| match response {
                Ok(res) => {
                    let rss_channel = rss::Channel::read_from(&res[..]);
                    if rss_channel.is_err() {
                        return None;
                    }
                    let rss_channel = rss_channel.unwrap();

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

        let podcasts_list_file =
            file_system::FileSystem::open_podcasts_list(&self.config.app_directory)?;

        let mut writer = csv::Writer::from_writer(podcasts_list_file);
        for podcast in podcasts {
            writer.serialize(podcast)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Remove the passed podcasts from the "podcast_list.csv" file which is located in the
    /// PODCASTS_DIR directory. does nothing if the passed values are not present in the file
    fn remove(&self, _remove_values: &Values) -> Result<(), Errors> {
        unimplemented!();
    }

    /// Lists the saved podcasts
    fn list(&self) -> Result<(), Errors> {
        unimplemented!();
    }
}
