use std::{fmt, fs, io, path::Path};

#[derive(Debug)]
pub enum FileSystemErrors {
    CreateDirectory(io::Error),
    CreateFile(io::Error),
    RenameError(io::Error),
    RemoveError(io::Error),
}

impl fmt::Display for FileSystemErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = String::new();

        match self {
            FileSystemErrors::CreateDirectory(error) => {
                message = format!("Can't create directory. {}", error);
            }
            FileSystemErrors::CreateFile(error) => {
                message = format!("Can't create file. {}", error);
            }
            FileSystemErrors::RenameError(error) => {
                message = format!("Can't rename file, {}", error);
            }
            FileSystemErrors::RemoveError(error) => {
                message = format!("Can't remove file. {}", error);
            }
        }

        write!(f, "{}", message)
    }
}

#[derive(Debug, PartialEq)]
pub enum FilePermissions {
    Read,
    Write,
    WriteTruncate,
    Append,
}

pub struct FileSystem<'a, 'b> {
    directory: &'a Path,
    file_name: &'b str,
    permissions: Vec<FilePermissions>,
}

impl<'a, 'b> FileSystem<'a, 'b> {
    pub fn new(directory: &'a Path, file_name: &'b str, permissions: Vec<FilePermissions>) -> Self {
        Self {
            directory,
            file_name,
            permissions,
        }
    }

    pub fn open(&self) -> Result<fs::File, FileSystemErrors> {
        let file_path = format!("{}/{}", self.directory.display(), self.file_name);
        let mut file = fs::OpenOptions::new();

        for permission in &self.permissions {
            if *permission == FilePermissions::Read {
                file.read(true);
            }

            if *permission == FilePermissions::Write {
                file.write(true);
            }

            if *permission == FilePermissions::WriteTruncate {
                file.write(true);
                file.truncate(true);
            }

            if *permission == FilePermissions::Append {
                file.append(true);
            }
        }

        if let Ok(file) = file.open(&file_path) {
            return Ok(file);
        }

        let directory = fs::create_dir_all(self.directory);
        if let Err(err) = directory {
            return Err(FileSystemErrors::CreateDirectory(err));
        }

        // If the file doesn't exist, it will always be in write mode and not append
        fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&file_path)
            .map_err(|error| FileSystemErrors::CreateFile(error))
    }

    #[allow(dead_code)]
    pub fn rename(&mut self, new_name: &'static str) -> Result<(), FileSystemErrors> {
        let old_path = format!("{}/{}", self.directory.display(), self.file_name);
        let new_path = format!("{}/{}", self.directory.display(), new_name);

        return match fs::rename(old_path, new_path) {
            Ok(_) => {
                self.file_name = new_name;
                Ok(())
            }
            Err(error) => Err(FileSystemErrors::RenameError(error)),
        };
    }

    #[allow(dead_code)]
    pub fn remove(self) -> Result<(), FileSystemErrors> {
        let path = format!("{}/{}", self.directory.display(), self.file_name);

        fs::remove_file(path).map_err(|error| FileSystemErrors::RemoveError(error))
    }
}
