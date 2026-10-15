use deskdeck_protocol::deck::{Action as ProtocolAction, Button, Deck, Grid, Page};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use thiserror::Error;

use crate::domain::{DomainError, Profile};

#[derive(Serialize)]
struct DeckFileOutput<'a> {
    deck: DeckMetadataOutput<'a>,
    #[serde(rename = "page")]
    pages: Vec<PageFileOutput<'a>>,
}

#[derive(Serialize)]
struct DeckMetadataOutput<'a> {
    name: &'a str,
    grid: &'a Grid,
}

#[derive(Serialize)]
struct PageFileOutput<'a> {
    name: &'a str,
    #[serde(rename = "button")]
    buttons: Vec<ButtonFileOutput<'a>>,
}

#[derive(Serialize)]
struct ButtonFileOutput<'a> {
    id: &'a str,
    label: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: &'a Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background_image: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_css: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_html: &'a Option<String>,
    on_press: &'a [ProtocolAction],
}

#[derive(Deserialize)]
struct DeckFile {
    deck: DeckMetadata,
    #[serde(rename = "page")]
    pages: Vec<PageFile>,
}

#[derive(Deserialize)]
struct PageFile {
    name: String,
    #[serde(rename = "button")]
    buttons: Vec<Button>,
}

#[derive(Deserialize)]
struct DeckMetadata {
    name: String,
    grid: Grid,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to parse deck: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize default deck: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("invalid profile: {0}")]
    Domain(#[from] DomainError),
}

pub fn deck_from_file_or_create(path: &str) -> Result<Profile, ConfigError> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            let deck = deck_default_home();
            let contents = toml::to_string_pretty(&deck_file_output(&deck))?;
            file.write_all(contents.as_bytes())?;
            file.flush()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    deck_from_file(path)
}

fn deck_file_output(deck: &Deck) -> DeckFileOutput<'_> {
    DeckFileOutput {
        deck: DeckMetadataOutput {
            name: &deck.name,
            grid: &deck.grid,
        },
        pages: deck
            .pages
            .iter()
            .map(|page| PageFileOutput {
                name: &page.name,
                buttons: page
                    .buttons
                    .iter()
                    .map(|button| ButtonFileOutput {
                        id: &button.id,
                        label: &button.label,
                        icon: &button.icon,
                        position: &button.position,
                        background_image: &button.background_image,
                        custom_css: &button.custom_css,
                        custom_html: &button.custom_html,
                        on_press: &button.on_press,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn deck_from_file(path: &str) -> Result<Profile, ConfigError> {
    let contents = std::fs::read_to_string(path)?;
    let file: DeckFile = toml::from_str(&contents)?;
    Ok(Profile::try_from(Deck {
        name: file.deck.name,
        grid: file.deck.grid,
        pages: file
            .pages
            .into_iter()
            .map(|page| Page {
                name: page.name,
                buttons: page.buttons,
            })
            .collect(),
    })?)
}

pub fn deck_to_file(path: &str, profile: &Profile) -> Result<(), ConfigError> {
    let deck = profile.to_deck();
    let contents = toml::to_string_pretty(&deck_file_output(&deck))?;
    let path = std::path::Path::new(path);
    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, contents)?;
    std::fs::rename(temp_path, path)?;
    Ok(())
}

pub fn deck_default_home() -> Deck {
    Deck {
        name: "Main".to_string(),
        grid: Grid { cols: 5, rows: 3 },
        pages: vec![Page {
            name: "Home".to_string(),
            buttons: vec![
                Button {
                    id: "home".to_string(),
                    label: "Desktop".to_string(),
                    icon: Some("fa-solid fa-desktop".to_string()),
                    position: None,
                    background_image: None,
                    custom_css: None,
                    custom_html: None,
                    on_press: vec![ProtocolAction {
                        kind: "hotkey".to_string(),
                        config: serde_json::from_value(serde_json::json!({
                            "keys": "win, d"
                        }))
                        .expect("default action config should be valid"),
                    }],
                },
                Button {
                    id: "mute".to_string(),
                    label: "Mute mic".to_string(),
                    icon: Some("fa-solid fa-microphone-slash".to_string()),
                    position: None,
                    background_image: None,
                    custom_css: None,
                    custom_html: None,
                    on_press: vec![ProtocolAction {
                        kind: "hotkey".to_string(),
                        config: serde_json::from_value(serde_json::json!({
                            "keys": "ctrl, shift, m"
                        }))
                        .expect("default action config should be valid"),
                    }],
                },
                Button {
                    id: "web".to_string(),
                    label: "Browser".to_string(),
                    icon: Some("fa-solid fa-globe".to_string()),
                    position: None,
                    background_image: None,
                    custom_css: None,
                    custom_html: None,
                    on_press: vec![ProtocolAction {
                        kind: "url".to_string(),
                        config: serde_json::from_value(serde_json::json!({
                            "url": "https://example.com"
                        }))
                        .expect("default action config should be valid"),
                    }],
                },
            ],
        }]
    }
}

pub fn profile_default_home() -> Result<Profile, ConfigError> {
    Profile::try_from(deck_default_home()).map_err(Into::into)
}

pub fn find_button<'a>(deck: &'a Deck, id: &'a str) -> Option<&'a Button> {
    deck.pages.iter().flat_map(|p| p.buttons.iter()).find(|b| b.id == id)
}

#[cfg(test)]
mod tests {
    use super::{deck_default_home, deck_from_file, deck_from_file_or_create, deck_to_file};
    use crate::domain::{Profile, Slot};
    use std::fs;

    fn test_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("deskdeck-{name}-{}.toml", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn creates_default_deck_file_when_missing() {
        let path = test_path("generated");
        let _ = fs::remove_file(&path);

        let deck = deck_from_file_or_create(&path).expect("missing deck should be generated");
        assert_eq!(deck.name, "Main");
        assert_eq!(
            deck.pages[0]
                .slots
                .iter()
                .filter(|slot| matches!(slot, Slot::Button(_)))
                .count(),
            3
        );
        assert_eq!(
            match &deck.pages[0].slots[0] {
                Slot::Button(button) => button.appearance.icon.as_deref(),
                Slot::Empty => None,
            },
            Some("fa-solid fa-desktop")
        );
        let default_deck = Profile::try_from(deck_default_home()).unwrap();
        assert_eq!(
            match &deck.pages[0].slots[1] {
                Slot::Button(button) => button.id.as_str(),
                Slot::Empty => "",
            },
            match &default_deck.pages[0].slots[1] {
                Slot::Button(button) => button.id.as_str(),
                Slot::Empty => "",
            }
        );

        fs::remove_file(path).expect("generated test deck should be removed");
    }

    #[test]
    fn keeps_existing_deck_file_unchanged() {
        let path = test_path("existing");
        let contents = "[deck]\nname = \"Custom\"\ngrid = { cols = 1, rows = 1 }\n\n[[page]]\nname = \"Home\"\n\n[[page.button]]\nid = \"custom\"\nlabel = \"Keep\"\non_press = []\n";
        fs::write(&path, contents).expect("test deck should be written");

        let deck = deck_from_file_or_create(&path).expect("existing deck should load");
        assert_eq!(deck.name, "Custom");
        assert!(matches!(
            &deck.pages[0].slots[0],
            Slot::Button(button) if button.id.as_str() == "custom"
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);

        fs::remove_file(path).expect("test deck should be removed");
    }

    #[test]
    fn loads_legacy_flat_action_settings() {
        let path = test_path("legacy-action");
        let contents = r#"
[deck]
name = "Legacy"
grid = { cols = 1, rows = 1 }

[[page]]
name = "Home"

[[page.button]]
id = "shortcut"
label = "Shortcut"
on_press = [{ action = "hotkey", keys = ["ctrl", "d"] }]
"#;
        fs::write(&path, contents).expect("legacy test deck should be written");

        let deck = deck_from_file(&path).expect("legacy action should load");

        assert_eq!(
            match &deck.pages[0].slots[0] {
                Slot::Button(button) => button.actions[0].config["keys"].as_str(),
                Slot::Empty => None,
            },
            Some("ctrl, d")
        );
        fs::remove_file(path).expect("legacy test deck should be removed");
    }

    #[test]
    fn saves_deck_and_can_load_it_again() {
        let path = test_path("saved");
        let mut deck = Profile::try_from(deck_default_home()).unwrap();
        let Slot::Button(button) = &mut deck.pages[0].slots[0] else {
            panic!("default button should occupy first slot");
        };
        button.appearance.background_image =
            Some("https://example.com/button.jpg".to_string());
        button.appearance.custom_css =
            Some("background-color: #243447; color: white;".to_string());
        button.appearance.custom_html =
            Some("<strong>LIVE</strong><span>Scene 1</span>".to_string());
        let (first, rest) = deck.pages[0].slots.split_at_mut(7);
        std::mem::swap(&mut first[0], &mut rest[0]);

        deck_to_file(&path, &deck).expect("deck should save");
        let loaded = deck_from_file(&path).expect("saved deck should load");

        assert_eq!(loaded.name, deck.name);
        assert_eq!(loaded, deck);

        fs::remove_file(path).expect("saved test deck should be removed");
    }
}
