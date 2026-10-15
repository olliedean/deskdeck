use super::DomainError;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use uuid::{Builder, Bytes, Uuid};

macro_rules! define_uuid_id {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name {
            uuid: Uuid,
            original_string: String,
        }

        impl $name {
            pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
                let value = value.as_ref();
                if value.trim().is_empty() || value.chars().any(char::is_control) {
                    return Err(DomainError::InvalidId { kind: $kind });
                }

                static NAMESPACE: Lazy<Uuid> = Lazy::new(|| {
                    Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
                        .expect("Invalid UUID namespace")
                });

                let mut hasher = Sha1::new();
                hasher.update(NAMESPACE.as_bytes());
                hasher.update(value.as_bytes());
                let result = hasher.finalize();
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&result[..16]);
                let bytes: Bytes = bytes;
                let uuid = Builder::from_sha1_bytes(bytes).into_uuid();

                Ok(Self {
                    uuid,
                    original_string: value.to_string(),
                })
            }

            pub fn as_str(&self) -> String {
                self.original_string.clone()
            }

            pub fn into_string(self) -> String {
                self.original_string
            }

            pub fn as_uuid(&self) -> Uuid {
                self.uuid
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.original_string)
            }
        }
    };
}

define_uuid_id!(ProfileId, "profile");
define_uuid_id!(PageId, "page");
define_uuid_id!(ButtonId, "button");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation_is_deterministic() {
        let id1 = ProfileId::new("test-string").unwrap();
        let id2 = ProfileId::new("test-string").unwrap();
        assert_eq!(id1.uuid, id2.uuid);
        assert_eq!(id1.original_string, id2.original_string);
        assert_eq!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_different_strings_produce_different_uuids() {
        let id1 = ProfileId::new("string1").unwrap();
        let id2 = ProfileId::new("string2").unwrap();
        assert_ne!(id1.uuid, id2.uuid);
        assert_ne!(id1.original_string, id2.original_string);
        assert_ne!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_save_load_consistency() {
        let original_string = "test-button";

        let original_id = ProfileId::new(original_string).unwrap();

        let saved_string = original_id.as_str();

        let loaded_id = ProfileId::new(&saved_string).unwrap();

        assert_eq!(original_id.uuid, loaded_id.uuid);
        assert_eq!(original_id.original_string, loaded_id.original_string);
        assert_eq!(original_id.as_str(), loaded_id.as_str());
    }
}
