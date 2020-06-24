pub struct FileSystem;
use std::{fs, io, path::Path};

const PODCAST_LIST_FILE: &'static str = "podcast_list.csv";

#[derive(Debug)]
pub enum FileSystemErrors {
    CreateAppDirectory(io::Error),
    CreatePodcastsFile(io::Error),
}

#[derive(Debug, PartialEq)]
pub enum FilePermissions {
    Read,
    Write,
    WriteTruncate,
    Append,
}

impl FileSystem {
    pub fn open_podcasts_list(
        app_directory: &Path,
        permissions: Vec<FilePermissions>,
    ) -> Result<fs::File, FileSystemErrors> {
        let file_path = format!("{}/{}", app_directory.display(), PODCAST_LIST_FILE);
        let mut file = fs::OpenOptions::new();

        for permission in permissions {
            if permission == FilePermissions::Read {
                file.read(true);
            }

            if permission == FilePermissions::Write {
                file.write(true);
            }

            if permission == FilePermissions::WriteTruncate {
                file.write(true);
                file.truncate(true);
            }

            if permission == FilePermissions::Append {
                file.append(true);
            }
        }

        if let Ok(file) = file.open(&file_path) {
            return Ok(file);
        }

        let directory = fs::create_dir_all(app_directory);
        if let Err(err) = directory {
            return Err(FileSystemErrors::CreateAppDirectory(err));
        }

        // If the file doesn't exist, it will always be in write mode and not append
        fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&file_path)
            .map_err(|error| FileSystemErrors::CreatePodcastsFile(error))
    }
}
