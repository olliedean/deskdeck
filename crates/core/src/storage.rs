use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("destination path has no file name: {path}")]
    InvalidDestination { path: PathBuf },
    #[error("failed to create temporary file for {destination}: {source}")]
    CreateTemporaryFile {
        destination: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write temporary file {path}: {source}")]
    WriteTemporaryFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to flush temporary file {path}: {source}")]
    FlushTemporaryFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to sync temporary file {path}: {source}")]
    SyncTemporaryFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to replace {destination} with {temporary}: {source}")]
    ReplaceDestination {
        temporary: PathBuf,
        destination: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub fn atomic_write(destination: impl AsRef<Path>, bytes: &[u8]) -> Result<(), StorageError> {
    atomic_write_with(
        destination.as_ref(),
        bytes,
        |file, temporary_path, bytes| {
            file.write_all(bytes)
                .map_err(|source| StorageError::WriteTemporaryFile {
                    path: temporary_path.to_path_buf(),
                    source,
                })?;
            file.flush()
                .map_err(|source| StorageError::FlushTemporaryFile {
                    path: temporary_path.to_path_buf(),
                    source,
                })?;
            file.sync_all()
                .map_err(|source| StorageError::SyncTemporaryFile {
                    path: temporary_path.to_path_buf(),
                    source,
                })
        },
    )
}

fn atomic_write_with(
    destination: &Path,
    bytes: &[u8],
    write_bytes: impl FnOnce(&mut File, &Path, &[u8]) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    if destination.file_name().is_none() {
        return Err(StorageError::InvalidDestination {
            path: destination.to_path_buf(),
        });
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (temporary_path, mut temporary_file) = create_temporary_file(parent, destination)?;

    let write_result = write_bytes(&mut temporary_file, &temporary_path, bytes);
    drop(temporary_file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(source) = fs::rename(&temporary_path, destination) {
        let _ = fs::remove_file(&temporary_path);
        return Err(StorageError::ReplaceDestination {
            temporary: temporary_path,
            destination: destination.to_path_buf(),
            source,
        });
    }

    Ok(())
}

fn create_temporary_file(
    parent: &Path,
    destination: &Path,
) -> Result<(PathBuf, File), StorageError> {
    loop {
        let temporary_name = format!(".deskdeck-{}.tmp", Uuid::new_v4());
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(StorageError::CreateTemporaryFile {
                    destination: destination.to_path_buf(),
                    source,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, atomic_write_with, StorageError};
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_PATH: AtomicU64 = AtomicU64::new(0);

    fn temporary_directory() -> PathBuf {
        let id = NEXT_TEMP_PATH.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("deskdeck-atomic-write-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        path
    }

    fn directory_entry_count(path: &PathBuf) -> usize {
        fs::read_dir(path).unwrap().count()
    }

    #[test]
    fn creates_new_file() {
        let directory = temporary_directory();
        let destination = directory.join("settings.json");

        atomic_write(&destination, b"{\"mode\":\"new\"}").unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"{\"mode\":\"new\"}");
        assert_eq!(directory_entry_count(&directory), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn replaces_existing_file_and_leaves_no_temporary_file() {
        let directory = temporary_directory();
        let destination = directory.join("settings.json");
        fs::write(&destination, b"old data").unwrap();

        atomic_write(&destination, b"new data").unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"new data");
        assert_eq!(directory_entry_count(&directory), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn write_failure_preserves_destination_and_removes_temporary_file() {
        let directory = temporary_directory();
        let destination = directory.join("settings.json");
        fs::write(&destination, b"existing data").unwrap();

        let result = atomic_write_with(&destination, b"partial data", |file, path, _| {
            file.write_all(b"partial data").unwrap();
            Err(StorageError::WriteTemporaryFile {
                path: path.to_path_buf(),
                source: io::Error::new(io::ErrorKind::Other, "simulated write failure"),
            })
        });

        assert!(matches!(
            result,
            Err(StorageError::WriteTemporaryFile { .. })
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"existing data");
        assert_eq!(directory_entry_count(&directory), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rename_failure_removes_temporary_file_and_preserves_destination() {
        let directory = temporary_directory();
        let destination = directory.join("settings.json");
        fs::create_dir(&destination).unwrap();
        let existing_child = destination.join("existing");
        fs::write(&existing_child, b"existing data").unwrap();

        let result = atomic_write(&destination, b"new data");

        assert!(matches!(
            result,
            Err(StorageError::ReplaceDestination { .. })
        ));
        assert_eq!(fs::read(&existing_child).unwrap(), b"existing data");
        assert_eq!(directory_entry_count(&directory), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reports_invalid_destination_directory() {
        let directory = temporary_directory();
        let destination = directory.join("missing").join("settings.json");

        let result = atomic_write(&destination, b"data");

        assert!(matches!(
            result,
            Err(StorageError::CreateTemporaryFile { .. })
        ));
        assert_eq!(directory_entry_count(&directory), 0);
        fs::remove_dir_all(directory).unwrap();
    }
}
