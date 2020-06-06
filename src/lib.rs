use clap::{self, App, Arg, ArgMatches};

mod podcasts;

#[derive(Debug)]
pub struct Application {
    matches: ArgMatches,
}

impl Application {
    pub fn new() -> Self {
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
                                .about(
                                    "List episodes. By default lists the episodes of all the podcasts",
                                )
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
                ).get_matches(),
        }
    }

    pub fn parse(&mut self) {
        if let Some(matches) = self.matches.subcommand_matches("podcasts") {
            podcasts::Podcasts::new(matches).run();
        }

        if let Some(ref _matches) = &self.matches.subcommand_matches("episodes") {
            unimplemented!();
        }
    }
}
