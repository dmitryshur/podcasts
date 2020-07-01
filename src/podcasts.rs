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

            println!("Adding podcasts...");
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
                    let mut hasher = DefaultHasher::new();
                    rss_url.hash(&mut hasher);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use clap::{App, Arg};
    use std::path::PathBuf;

    fn create_config() -> Config {
        let app_directory = "/Users/dmitryshur/.podcasts";
        let download_directory = "/Users/dmitryshur/.podcasts/downloads";

        Config {
            app_directory: PathBuf::from(app_directory),
            download_directory: PathBuf::from(download_directory),
        }
    }

    fn create_app() -> App<'static> {
        App::new("pcasts").subcommand(
            App::new("podcasts")
                .arg(
                    Arg::with_name("list")
                        .about("Show a list of previously added RSS feeds")
                        .short('l')
                        .long("--list")
                        .conflicts_with_all(&["add", "remove"]),
                )
                .arg(
                    Arg::with_name("add")
                        .about("Add new RSS feed")
                        .short('a')
                        .long("--add")
                        .takes_value(true)
                        .multiple(true)
                        .conflicts_with_all(&["list", "remove"]),
                )
                .arg(
                    Arg::with_name("remove")
                        .about("Remove an existing RSS feed")
                        .short('r')
                        .long("--remove")
                        .takes_value(true)
                        .multiple(true)
                        .conflicts_with_all(&["list", "add"]),
                ),
        )
    }

    #[test]
    fn podcasts_add_single() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--add",
            "http://feeds.feedburner.com/Http203Podcast",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        // We pass an empty reader, so the headers line should be added
        let input = String::new();
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
"###;

        podcasts
            .add(&podcast_matches.values_of("add").unwrap(), input, &mut output)
            .expect("Can't add podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }

    #[test]
    fn podcasts_add_multiple() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--add",
            "http://feeds.feedburner.com/Http203Podcast",
            "--add",
            "https://feed.syntax.fm/rss",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        // We pass an empty reader, so the headers line should be added
        let input = String::new();
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;

        podcasts
            .add(&podcast_matches.values_of("add").unwrap(), input, &mut output)
            .expect("Can't add podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }

    #[test]
    fn podcasts_add_append() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--add",
            "http://feeds.feedburner.com/Http203Podcast",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
"###;

        podcasts
            .add(&podcast_matches.values_of("add").unwrap(), input, &mut output)
            .expect("Can't add podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }

    #[test]
    fn podcasts_add_existing() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--add",
            "http://feeds.feedburner.com/Http203Podcast",
            "--add",
            "https://feed.syntax.fm/rss",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = "";

        podcasts
            .add(&podcast_matches.values_of("add").unwrap(), input, &mut output)
            .expect("Can't add podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }

    #[test]
    fn podcasts_list() {
        let args = create_app().get_matches_from(vec!["pcasts", "podcasts", "--list"]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let podcast = Podcast {
            id: 12772734294147401495,
            url: "https://developers.google.com/web/shows/http203/podcast/".to_string(),
            rss_url: "http://feeds.feedburner.com/Http203Podcast".to_string(),
            title: "HTTP 203".to_string(),
        };
        let expected_output = podcast.to_string();

        podcasts.list(input, &mut output).expect("Can't list podcasts");

        assert_eq!(std::str::from_utf8(&output).unwrap().trim(), expected_output.trim());
    }

    #[test]
    fn podcasts_list_multiple() {
        let args = create_app().get_matches_from(vec!["pcasts", "podcasts", "--list"]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let first_podcast = Podcast {
            id: 12772734294147401495,
            url: "https://developers.google.com/web/shows/http203/podcast/".to_string(),
            rss_url: "http://feeds.feedburner.com/Http203Podcast".to_string(),
            title: "HTTP 203".to_string(),
        };

        let second_podcast = Podcast {
            id: 15913066141282366353,
            url: "https://syntax.fm".to_string(),
            rss_url: "https://feed.syntax.fm/rss".to_string(),
            title: "Syntax - Tasty Web Development Treats".to_string(),
        };

        let expected_output = format!("{}\n{}", first_podcast, second_podcast);

        podcasts.list(input, &mut output).expect("Can't list podcasts");

        assert_eq!(std::str::from_utf8(&output).unwrap().trim(), expected_output.trim());
    }

    #[test]
    fn podcasts_remove() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--remove",
            "http://feeds.feedburner.com/Http203Podcast",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = r###"id,url,rss_url,title
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;

        podcasts
            .remove(&podcast_matches.values_of("remove").unwrap(), input, &mut output)
            .expect("Can't remove podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }

    #[test]
    fn podcasts_remove_multiple() {
        let args = create_app().get_matches_from(vec![
            "pcasts",
            "podcasts",
            "--remove",
            "http://feeds.feedburner.com/Http203Podcast",
            "--remove",
            "https://feed.syntax.fm/rss",
        ]);
        let podcast_matches = args.subcommand_matches("podcasts").expect("No podcasts matches");
        let config = create_config();
        let podcasts = Podcasts::new(&podcast_matches, &config);

        let input = r###"id,url,rss_url,title
12772734294147401495,https://developers.google.com/web/shows/http203/podcast/,http://feeds.feedburner.com/Http203Podcast,HTTP 203
15913066141282366353,https://syntax.fm,https://feed.syntax.fm/rss,Syntax - Tasty Web Development Treats
"###;
        let input = input.as_bytes();
        let mut output = Vec::new();
        let expected_output = "";

        podcasts
            .remove(&podcast_matches.values_of("remove").unwrap(), input, &mut output)
            .expect("Can't remove podcast");

        assert_eq!(std::str::from_utf8(&output).unwrap(), expected_output);
    }
}
