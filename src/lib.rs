use clap::{self, App, Arg, Values};
use csv;
use std::{fmt, io, path::PathBuf};

mod consts;
mod episodes;
mod file_system;
mod podcasts;
mod web;

#[derive(Debug)]
pub enum Errors {
    RSS,
    WrongID(u64),
    IO(io::Error),
    CSV(csv::Error),
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Errors::RSS => write!(f, "Couldn't parse RSS feed"),
            Errors::WrongID(id) => write!(f, "Invalid ID: {}", id),
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

impl Config {
    pub fn new(app_directory: PathBuf, download_directory: PathBuf) -> Self {
        Self {
            app_directory,
            download_directory,
        }
    }
}

pub struct ApplicationBuilder {
    config: Config,
    app: App<'static>,
    subcommands: Vec<App<'static>>,
}

impl ApplicationBuilder {
    pub fn new(config: Config) -> Self {
        let app = App::new("pcasts")
            .version("1.0.0")
            .author("Dmitry S. <dimashur@gmail.com>")
            .about("CLI util for downloading podcasts");

        Self {
            config,
            app,
            subcommands: vec![],
        }
    }

    pub fn podcasts_subcommand(mut self) -> Self {
        self.subcommands.push(
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
        );

        self
    }

    pub fn episodes_subcommand(mut self) -> Self {
        self.subcommands.push(
            App::new("episodes")
                .subcommand(
                    App::new("list")
                        .about("List episodes. By default lists the episodes of all the podcasts")
                        .arg(
                            Arg::with_name("id")
                                .about("Id of the podcast to list")
                                .long("--id")
                                .takes_value(true)
                                .multiple(true),
                        ),
                )
                .subcommand(
                    App::new("update").arg(
                        Arg::with_name("id")
                            .about("ID of the podcast to update")
                            .long("--id")
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
        );

        self
    }

    pub fn build(self) -> Application {
        let app = self.app.clone().subcommands(self.subcommands);

        Application::new(self.config, app)
    }
}

#[derive(Debug)]
pub struct Application {
    app: App<'static>,
    config: Config,
}

impl Application {
    pub fn new(config: Config, app: App<'static>) -> Self {
        Self { config, app }
    }

    pub fn run(&mut self) -> Result<(), Errors> {
        let matches = self.app.get_matches_mut();

        if let Some(matches) = matches.subcommand_matches("podcasts") {
            return podcasts::Podcasts::new(matches, &self.config).run();
        }

        if let Some(matches) = matches.subcommand_matches("episodes") {
            return episodes::Episodes::new(matches, &self.config).run();
        }

        Ok(())
    }
}
