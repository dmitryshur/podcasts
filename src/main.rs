use podcasts::{ApplicationBuilder, Config};
use std::{env, path::PathBuf};

fn main() {
    let home_directory = env::var("HOME").expect("Can't find $HOME dir variable");
    let app_directory = env::var("PODCASTS_DIR").unwrap_or(format!("{}/{}", home_directory.clone(), ".podcasts"));
    let download_directory = env::var("PODCASTS_DOWNLOAD_DIR").unwrap_or(format!("{}/Downloads", home_directory));

    let config = Config::new(PathBuf::from(app_directory), PathBuf::from(download_directory));
    let mut app = ApplicationBuilder::new(config)
        .podcasts_subcommand()
        .episodes_subcommand()
        .build();

    if let Err(error) = app.run() {
        eprintln!("{}", error);
        std::process::exit(1);
    }

    println!("Done");
}
