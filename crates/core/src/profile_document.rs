use crate::domain::Profile;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDocument {
    pub schema_version: u32,
    pub profile: Profile,
}

impl ProfileDocument {
    pub fn new(profile: Profile) -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            profile,
        }
    }

    pub fn to_json(&self) -> Result<String, ProfileDocumentError> {
        serde_json::to_string_pretty(self).map_err(ProfileDocumentError::Serialize)
    }

    pub fn from_json(json: &str) -> Result<Self, ProfileDocumentError> {
        let document: Self =
            serde_json::from_str(json).map_err(ProfileDocumentError::Deserialize)?;
        if document.schema_version != PROFILE_SCHEMA_VERSION {
            return Err(ProfileDocumentError::UnsupportedSchemaVersion {
                version: document.schema_version,
            });
        }
        Ok(document)
    }
}

#[derive(Debug, Error)]
pub enum ProfileDocumentError {
    #[error("failed to serialize profile document: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to deserialize profile document: {0}")]
    Deserialize(#[source] serde_json::Error),
    #[error("unsupported profile schema version: {version}")]
    UnsupportedSchemaVersion { version: u32 },
}

#[cfg(test)]
mod tests {
    use super::{ProfileDocument, ProfileDocumentError, PROFILE_SCHEMA_VERSION};
    use crate::domain::{
        ActionInstance, Border, BorderStyle, Button, ButtonAppearance, ButtonContent, ButtonId,
        ButtonState, GridSize, Page, PageId, Profile, Slot,
    };
    use serde_json::json;

    fn representative_profile() -> Profile {
        let mut profile = Profile::new("Studio").unwrap();
        let primary_grid = GridSize::new(5, 3).unwrap();
        let compact_grid = GridSize::new(2, 2).unwrap();
        profile.grid = primary_grid;
        profile.pages[0].grid_size = primary_grid;

        let mut primary_button = Button::new(
            ButtonId::new("primary").unwrap(),
            "Primary action".to_string(),
        );
        primary_button.appearance = ButtonAppearance {
            icon: Some("fa-solid fa-bolt".to_string()),
            background_color: Some("#203040".to_string()),
            text_color: Some("#ffffff".to_string()),
            border: Some(Border {
                width: 2,
                color: "#80c0ff".to_string(),
                style: BorderStyle::Solid,
            }),
            ..ButtonAppearance::default()
        };
        primary_button.content = ButtonContent::Custom {
            markup: "<strong>Ready</strong>".to_string(),
            css: Some("color: #80c0ff;".to_string()),
        };
        primary_button.state = ButtonState::ToggledOn;
        primary_button.actions = vec![
            ActionInstance::new("hotkey", json!({"keys": ["ctrl", "shift", "p"]})).unwrap(),
            ActionInstance::new("type", json!({"text": "launch"})).unwrap(),
        ];
        profile.pages[0].slots[4] = Slot::Button(primary_button);

        let mut compact_page = Page::new(
            PageId::new("compact").unwrap(),
            "Compact tools",
            compact_grid,
        )
        .unwrap();
        let mut compact_button = Button::new(
            ButtonId::new("compact-action").unwrap(),
            "Compact action".to_string(),
        );
        compact_button.appearance.icon = Some("fa-solid fa-wrench".to_string());
        compact_button.state = ButtonState::Disabled;
        compact_button.actions =
            vec![ActionInstance::new("url", json!({"url": "https://example.com"})).unwrap()];
        compact_page
            .add_button(compact_button, 3)
            .expect("button should fit the compact page");
        profile.pages.push(compact_page);

        profile
    }

    #[test]
    fn round_trips_representative_profile_as_pretty_json() {
        let profile = representative_profile();
        let document = ProfileDocument::new(profile.clone());

        let json = document.to_json().unwrap();
        let restored = ProfileDocument::from_json(&json).unwrap();

        assert_eq!(restored.schema_version, PROFILE_SCHEMA_VERSION);
        assert_eq!(restored.profile, profile);
        assert!(json.contains("\n  \"schema_version\""));
        assert_eq!(
            restored.profile.pages[0].grid_size,
            GridSize::new(5, 3).unwrap()
        );
        assert_eq!(
            restored.profile.pages[1].grid_size,
            GridSize::new(2, 2).unwrap()
        );
    }

    #[test]
    fn serialized_document_includes_schema_version() {
        let json = ProfileDocument::new(representative_profile())
            .to_json()
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["schema_version"], json!(1));
    }

    #[test]
    fn rejects_unknown_document_fields() {
        let mut value =
            serde_json::to_value(ProfileDocument::new(representative_profile())).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".to_string(), json!(true));

        assert!(matches!(
            ProfileDocument::from_json(&value.to_string()),
            Err(ProfileDocumentError::Deserialize(_))
        ));
    }

    #[test]
    fn rejects_unsupported_schema_versions() {
        let mut value =
            serde_json::to_value(ProfileDocument::new(representative_profile())).unwrap();
        value["schema_version"] = json!(2);

        assert!(matches!(
            ProfileDocument::from_json(&value.to_string()),
            Err(ProfileDocumentError::UnsupportedSchemaVersion { version: 2 })
        ));
    }
}
