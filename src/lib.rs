use clap::{self, App, Arg, ArgMatches};
use csv;
use std::{env, fmt, io, path::PathBuf};

mod consts;
mod file_system;
mod podcasts;
mod web;

#[derive(Debug)]
pub enum Errors {
    RSS,
    IO(io::Error),
    CSV(csv::Error),
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Errors::RSS => write!(f, "Couldn't parse RSS feed"),
            Errors::IO(ref e) => write!(f, "IO error: {}", e),
            Errors::CSV(ref e) => write!(f, "CSV error: {}", e),
        }
    }
}

impl From<csv::Error> for Errors {
    fn from(err: csv::Error) -> Errors {
        Errors::CSV(err)
    }
}

impl From<file_system::FileSystemErrors> for Errors {
    fn from(err: file_system::FileSystemErrors) -> Errors {
        match err {
            file_system::FileSystemErrors::CreateFile(e) => Errors::IO(e),
            file_system::FileSystemErrors::CreateDirectory(e) => Errors::IO(e),
            file_system::FileSystemErrors::RenameError(e) => Errors::IO(e),
            file_system::FileSystemErrors::RemoveError(e) => Errors::IO(e),
        }
    }
}

impl From<io::Error> for Errors {
    fn from(err: io::Error) -> Errors {
        Errors::IO(err)
    }
}

#[derive(Debug)]
pub struct Config {
    app_directory: PathBuf,
    download_directory: PathBuf,
}

#[derive(Debug)]
pub struct Application {
    matches: ArgMatches,
    config: Config,
}

impl Application {
    /// Constructs a new Application.
    /// PODCASTS_DIR is used as the main directory for the application. by default it is set
    /// to the .podcasts directory in the HOME directory of the user
    /// PODCASTS_DOWNLOAD_DIR is used as the directory where all the podcasts are downloaded to
    pub fn new() -> Self {
        let home_directory = env::var("HOME").expect("Can't find $HOME dir variable");
        let app_directory = env::var("PODCASTS_DIR").unwrap_or(format!("{}/{}", home_directory.clone(), ".podcasts"));

        let download_directory = env::var("PODCASTS_DOWNLOAD_DIR").unwrap_or(format!("{}/Downloads", home_directory));

        let config = Config {
            app_directory: PathBuf::from(app_directory),
            download_directory: PathBuf::from(download_directory),
        };

        Self {
            matches: App::new("pcasts")
                .version("1.0.0")
                .author("Dmitry S. <dimashur@gmail.com>")
                .about("CLI util for downloading podcasts")
                .subcommand(
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
                .subcommand(
                    App::new("episodes")
                        .subcommand(
                            App::new("list")
                                .about("List episodes. By default lists the episodes of all the podcasts")
                                .arg(
                                    Arg::with_name("name")
                                        .about("Name of the podcast to list")
                                        .long("--name")
                                        .takes_value(true)
                                        .multiple(true),
                                ),
                        )
                        .subcommand(
                            App::new("update").arg(
                                Arg::with_name("name")
                                    .about("Name of the podcast to update")
                                    .long("--name")
                                    .multiple(true)
                                    .takes_value(true),
                            ),
                        )
                        .subcommand(
                            App::new("download")
                                .arg(
                                    Arg::with_name("podcast")
                                        .about("Name of the podcast")
                                        .long("--podcast")
                                        .required(true)
                                        .takes_value(true),
                                )
                                .arg(
                                    Arg::with_name("name")
                                        .about("Names of the episodes to download")
                                        .long("--name")
                                        .multiple(true)
                                        .takes_value(true),
                                )
                                .arg(
                                    Arg::with_name("newest")
                                        .about("Download the newest episodes after the update")
                                        .takes_value(true)
                                        .conflicts_with("name")
                                        .long("--newest"),
                                )
                                .arg(
                                    Arg::with_name("list")
                                        .about("List the downloaded episodes of the provided podcast")
                                        .short('l')
                                        .long("--list")
                                        .conflicts_with("newest")
                                        .conflicts_with("name")
                                        .requires("podcast")
                                        .takes_value(true),
                                ),
                        )
                        .subcommand(
                            App::new("remove").arg(
                                Arg::with_name("name")
                                    .about("Names of the episodes to remove")
                                    .long("--name")
                                    .multiple(true)
                                    .takes_value(true),
                            ),
                        )
                        .subcommand(
                            App::new("archive")
                                .arg(
                                    Arg::with_name("podcast")
                                        .about("The name of the podcast")
                                        .long("--podcast")
                                        .required(true)
                                        .takes_value(true),
                                )
                                .arg(
                                    Arg::with_name("list")
                                        .about("List archived episodes")
                                        .short('l')
                                        .long("--list"),
                                )
                                .arg(
                                    Arg::with_name("add")
                                        .about("Add an episode to the archive")
                                        .short('a')
                                        .long("--add")
                                        .conflicts_with("list"),
                                )
                                .arg(
                                    Arg::with_name("remove-download")
                                        .about("Remove the added episode from the downloads")
                                        .long("--remove-download")
                                        .requires("add"),
                                ),
                        ),
                )
                .get_matches(),
            config,
        }
    }

    /// Matches the passed sub commands. podcasts and episodes are the only options
    pub fn parse(&mut self) -> Result<(), Errors> {
        if let Some(matches) = self.matches.subcommand_matches("podcasts") {
            return podcasts::Podcasts::new(matches, &self.config).run();
        }

        if let Some(ref _matches) = &self.matches.subcommand_matches("episodes") {
            unimplemented!();
        }

        Ok(())
    }
}
