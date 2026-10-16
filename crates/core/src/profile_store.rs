use crate::domain::{DomainError, Profile, ProfileId};
use crate::profile_document::{ProfileDocument, ProfileDocumentError, PROFILE_SCHEMA_VERSION};
use crate::storage::{atomic_write, StorageError};
use crate::storage_paths::StoragePaths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: ProfileId,
    pub name: String,
    pub page_count: usize,
    pub button_count: usize,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    paths: StoragePaths,
}

impl ProfileStore {
    pub fn new(paths: StoragePaths) -> Self {
        Self { paths }
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileSummary>, ProfileStoreError> {
        let profiles_dir = self.paths.profiles_dir();
        let entries = match fs::read_dir(&profiles_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(ProfileStoreError::ListProfiles {
                    path: profiles_dir,
                    source,
                });
            }
        };

        let mut summaries = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| ProfileStoreError::ListProfiles {
                path: profiles_dir.clone(),
                source,
            })?;
            let path = entry.path();
            if !entry
                .file_type()
                .map_err(|source| ProfileStoreError::InspectProfile {
                    path: path.clone(),
                    source,
                })?
                .is_file()
                || path.extension().and_then(|extension| extension.to_str()) != Some("json")
            {
                continue;
            }

            let bytes = fs::read(&path).map_err(|source| ProfileStoreError::ReadProfile {
                path: path.clone(),
                source,
            })?;
            if let Ok(summary) = summarize_profile(&bytes) {
                let expected_file_name = format!("{}.json", summary.id.as_uuid());
                if path.file_name().and_then(|name| name.to_str())
                    == Some(expected_file_name.as_str())
                {
                    summaries.push(summary);
                }
            }
        }

        summaries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(summaries)
    }

    pub fn load_profile(&self, id: &ProfileId) -> Result<Profile, ProfileStoreError> {
        let path = self.profile_path(id);
        let bytes = fs::read(&path).map_err(|source| ProfileStoreError::ReadProfile {
            path: path.clone(),
            source,
        })?;
        let json =
            std::str::from_utf8(&bytes).map_err(|source| ProfileStoreError::InvalidUtf8 {
                path: path.clone(),
                source,
            })?;
        let document = ProfileDocument::from_json(json)?;
        if document.profile.id != *id {
            return Err(ProfileStoreError::ProfileIdMismatch { path });
        }
        document.profile.validate()?;
        Ok(document.profile)
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), ProfileStoreError> {
        profile.validate()?;
        let path = self.profile_path(&profile.id);
        let json = ProfileDocument::new(profile.clone()).to_json()?;
        fs::create_dir_all(self.paths.profiles_dir()).map_err(|source| {
            ProfileStoreError::CreateDirectory {
                path: self.paths.profiles_dir(),
                source,
            }
        })?;
        atomic_write(path, json.as_bytes())?;
        Ok(())
    }

    pub fn delete_profile(&self, id: &ProfileId) -> Result<PathBuf, ProfileStoreError> {
        let profile_path = self.profile_path(id);
        let contents =
            fs::read(&profile_path).map_err(|source| ProfileStoreError::ReadProfile {
                path: profile_path.clone(),
                source,
            })?;
        fs::create_dir_all(self.paths.backups_dir()).map_err(|source| {
            ProfileStoreError::CreateDirectory {
                path: self.paths.backups_dir(),
                source,
            }
        })?;
        let backup_path =
            self.paths
                .backups_dir()
                .join(format!("{}-{}.json", id.as_uuid(), Uuid::new_v4()));
        atomic_write(&backup_path, &contents)?;
        fs::remove_file(&profile_path).map_err(|source| ProfileStoreError::DeleteProfile {
            path: profile_path,
            source,
        })?;
        Ok(backup_path)
    }

    fn profile_path(&self, id: &ProfileId) -> PathBuf {
        self.paths
            .profiles_dir()
            .join(format!("{}.json", id.as_uuid()))
    }
}

#[derive(Debug, Error)]
pub enum ProfileStoreError {
    #[error("failed to list profiles in {path}: {source}")]
    ListProfiles {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to inspect profile file {path}: {source}")]
    InspectProfile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read profile file {path}: {source}")]
    ReadProfile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("profile file is not valid UTF-8: {path}: {source}")]
    InvalidUtf8 {
        path: PathBuf,
        #[source]
        source: std::str::Utf8Error,
    },
    #[error("profile ID in {path} does not match the requested ID")]
    ProfileIdMismatch { path: PathBuf },
    #[error("failed to create storage directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to delete profile file {path}: {source}")]
    DeleteProfile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    ProfileDocument(#[from] ProfileDocumentError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Domain(#[from] DomainError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryDocument {
    schema_version: u32,
    profile: SummaryProfile,
}

#[derive(Deserialize)]
struct SummaryProfile {
    id: ProfileId,
    name: String,
    created_at: SystemTime,
    updated_at: SystemTime,
    pages: Vec<SummaryPage>,
}

#[derive(Deserialize)]
struct SummaryPage {
    slots: Vec<SummarySlot>,
}

#[derive(Deserialize)]
enum SummarySlot {
    Empty,
    Button(serde::de::IgnoredAny),
}

fn summarize_profile(bytes: &[u8]) -> Result<ProfileSummary, ProfileDocumentError> {
    let document: SummaryDocument =
        serde_json::from_slice(bytes).map_err(ProfileDocumentError::Deserialize)?;
    if document.schema_version != PROFILE_SCHEMA_VERSION {
        return Err(ProfileDocumentError::UnsupportedSchemaVersion {
            version: document.schema_version,
        });
    }
    let button_count = document
        .profile
        .pages
        .iter()
        .flat_map(|page| &page.slots)
        .filter(|slot| matches!(slot, SummarySlot::Button(_)))
        .count();
    Ok(ProfileSummary {
        id: document.profile.id,
        name: document.profile.name,
        page_count: document.profile.pages.len(),
        button_count,
        created_at: document.profile.created_at,
        updated_at: document.profile.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::{ProfileStore, ProfileStoreError};
    use crate::domain::{Button, ButtonId, Page, PageId, Profile, Slot};
    use crate::profile_document::ProfileDocument;
    use crate::storage_paths::StoragePaths;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_PATH: AtomicU64 = AtomicU64::new(0);

    fn store() -> (PathBuf, ProfileStore) {
        let id = NEXT_TEMP_PATH.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "deskdeck-profile-store-{}-{id}",
            std::process::id()
        ));
        let paths = StoragePaths::with_base_dir(root.join("data"));
        (root, ProfileStore::new(paths))
    }

    fn profile(name: &str) -> Profile {
        Profile::new(name).unwrap()
    }

    fn insert_button(profile: &mut Profile, page_index: usize, id: &str) {
        profile.pages[page_index].slots[0] =
            Slot::Button(Button::new(ButtonId::new(id).unwrap(), id.to_string()));
    }

    #[test]
    fn saves_and_loads_profile() {
        let (root, store) = store();
        let mut expected = profile("Lighting");
        insert_button(&mut expected, 0, "scene-one");

        store.save_profile(&expected).unwrap();
        let loaded = store.load_profile(&expected.id).unwrap();

        assert_eq!(loaded, expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lists_profiles_with_summary_counts() {
        let (root, store) = store();
        let mut first = profile("Alpha");
        insert_button(&mut first, 0, "alpha-button");
        let mut second = profile("Beta");
        let second_page = Page::new(PageId::new("tools").unwrap(), "Tools", second.grid).unwrap();
        second.add_page(second_page).unwrap();
        insert_button(&mut second, 0, "beta-button-one");
        insert_button(&mut second, 1, "beta-button-two");
        store.save_profile(&second).unwrap();
        store.save_profile(&first).unwrap();

        let summaries = store.list_profiles().unwrap();

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].name, "Alpha");
        assert_eq!(summaries[0].id, first.id);
        assert_eq!(summaries[0].page_count, 1);
        assert_eq!(summaries[0].button_count, 1);
        assert_eq!(summaries[0].created_at, first.created_at);
        assert_eq!(summaries[0].updated_at, first.updated_at);
        assert_eq!(summaries[1].name, "Beta");
        assert_eq!(summaries[1].page_count, 2);
        assert_eq!(summaries[1].button_count, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deletes_profile_after_creating_backup() {
        let (root, store) = store();
        let profile = profile("Lighting");
        store.save_profile(&profile).unwrap();

        let backup_path = store.delete_profile(&profile.id).unwrap();

        assert!(!store
            .paths
            .profiles_dir()
            .join(format!("{}.json", profile.id.as_uuid()))
            .exists());
        let backup: ProfileDocument =
            ProfileDocument::from_json(&fs::read_to_string(backup_path).unwrap()).unwrap();
        assert_eq!(backup.profile, profile);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn returns_corrupt_profile_error_when_loading() {
        let (root, store) = store();
        let id = crate::domain::ProfileId::new("corrupt").unwrap();
        fs::create_dir_all(store.paths.profiles_dir()).unwrap();
        fs::write(
            store
                .paths
                .profiles_dir()
                .join(format!("{}.json", id.as_uuid())),
            b"{not json",
        )
        .unwrap();

        assert!(matches!(
            store.load_profile(&id),
            Err(ProfileStoreError::ProfileDocument(_))
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_profile_does_not_block_listing_or_loading_others() {
        let (root, store) = store();
        let valid_profile = profile("Valid");
        store.save_profile(&valid_profile).unwrap();
        let corrupt_id = crate::domain::ProfileId::new("corrupt").unwrap();
        fs::write(
            store
                .paths
                .profiles_dir()
                .join(format!("{}.json", corrupt_id.as_uuid())),
            b"{not json",
        )
        .unwrap();

        let summaries = store.list_profiles().unwrap();
        let loaded = store.load_profile(&valid_profile.id).unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, valid_profile.id);
        assert_eq!(loaded, valid_profile);
        fs::remove_dir_all(root).unwrap();
    }
}
