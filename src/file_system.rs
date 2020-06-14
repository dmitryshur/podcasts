pub struct FileSystem;
use std::path::PathBuf;
use std::{fs, io, path::Path};

const PODCAST_LIST_FILE: &'static str = "podcast_list.csv";

#[derive(Debug)]
pub enum FileSystemErrors {
    CreateAppDirectory(io::Error),
    CreatePodcastsFile(io::Error),
}

impl FileSystem {
    pub fn open(path: &Path) -> Result<fs::File, ()> {
        let directory = fs::File::open(path);
        match directory {
            Ok(file) => Ok(file),
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    println!("not found error");
                    Err(())
                }
                _ => {
                    println!("other error");
                    Err(())
                }
            },
        }
    }

    pub fn open_podcasts_list(app_directory: &Path) -> Result<fs::File, FileSystemErrors> {
        let file_path = format!("{}/{}", app_directory.display(), PODCAST_LIST_FILE);
        let file = fs::File::open(&file_path);
        if let Ok(file) = file {
            return Ok(file);
        }

        let directory = fs::create_dir_all(app_directory);
        if let Err(err) = directory {
            return Err(FileSystemErrors::CreateAppDirectory(err));
        }

        fs::File::create(&file_path).map_err(|error| FileSystemErrors::CreatePodcastsFile(error))
    }
}
