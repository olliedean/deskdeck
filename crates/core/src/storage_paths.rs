use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoragePathError {
    #[error(
        "could not resolve DeskDeck data directory; {variable} must be set to an absolute path"
    )]
    BaseDirectoryUnavailable { variable: &'static str },
    #[error("DeskDeck storage paths are unsupported on this platform")]
    UnsupportedPlatform,
    #[error("failed to create storage directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePaths {
    base_dir: PathBuf,
}

impl StoragePaths {
    pub fn resolve() -> Result<Self, StoragePathError> {
        Ok(Self {
            base_dir: platform_base_directory()?,
        })
    }

    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.base_dir.join("profiles")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.base_dir.join("backups")
    }

    pub fn ensure_directories(&self) -> Result<(), StoragePathError> {
        for path in [
            self.base_dir(),
            self.profiles_dir().as_path(),
            self.backups_dir().as_path(),
        ] {
            fs::create_dir_all(path).map_err(|source| StoragePathError::CreateDirectory {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn platform_base_directory() -> Result<PathBuf, StoragePathError> {
    if let Some(path) = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path.join("deskdeck"));
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".local").join("share").join("deskdeck"))
        .ok_or(StoragePathError::BaseDirectoryUnavailable {
            variable: "XDG_DATA_HOME or HOME",
        })
}

#[cfg(target_os = "macos")]
fn platform_base_directory() -> Result<PathBuf, StoragePathError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("DeskDeck")
        })
        .ok_or(StoragePathError::BaseDirectoryUnavailable { variable: "HOME" })
}

#[cfg(target_os = "windows")]
fn platform_base_directory() -> Result<PathBuf, StoragePathError> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("DeskDeck"))
        .ok_or(StoragePathError::BaseDirectoryUnavailable {
            variable: "APPDATA",
        })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_base_directory() -> Result<PathBuf, StoragePathError> {
    Err(StoragePathError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::{StoragePathError, StoragePaths};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_PATH: AtomicU64 = AtomicU64::new(0);

    fn temporary_path() -> PathBuf {
        let id = NEXT_TEMP_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "deskdeck-storage-paths-{}-{id}",
            std::process::id()
        ))
    }

    #[test]
    fn resolves_profiles_and_backups_paths_from_override() {
        let root = temporary_path();
        let base = root.join("data");
        let paths = StoragePaths::with_base_dir(&base);

        assert_eq!(paths.base_dir(), base);
        assert_eq!(paths.profiles_dir(), base.join("profiles"));
        assert_eq!(paths.backups_dir(), base.join("backups"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ensures_required_directories_exist() {
        let root = temporary_path();
        let paths = StoragePaths::with_base_dir(root.join("data"));

        paths.ensure_directories().unwrap();

        assert!(paths.base_dir().is_dir());
        assert!(paths.profiles_dir().is_dir());
        assert!(paths.backups_dir().is_dir());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolving_paths_does_not_create_directories() {
        let root = temporary_path();
        let paths = StoragePaths::with_base_dir(root.join("data"));

        assert!(!root.exists());
        assert_eq!(paths.profiles_dir(), root.join("data").join("profiles"));
        assert!(!root.exists());
    }

    #[test]
    fn reports_invalid_base_path_when_creating_directories() {
        let root = temporary_path();
        fs::create_dir_all(&root).unwrap();
        let base = root.join("file");
        fs::write(&base, "not a directory").unwrap();
        let paths = StoragePaths::with_base_dir(&base);

        let error = paths.ensure_directories().unwrap_err();

        assert!(matches!(
            error,
            StoragePathError::CreateDirectory { path, .. } if path == base
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
