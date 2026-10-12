use deskdeck_protocol::deck::{Action as ProtocolAction, Button, Deck, Grid, Page};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use thiserror::Error;

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
    on_press: Vec<ActionFileOutput<'a>>,
}

#[derive(Serialize)]
struct ActionFileOutput<'a> {
    #[serde(rename = "action")]
    kind: &'a str,
    keys: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    text: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: &'a Option<f64>,
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
}

pub fn deck_from_file_or_create(path: &str) -> Result<Deck, ConfigError> {
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
                        on_press: button
                            .on_press
                            .iter()
                            .map(|action| ActionFileOutput {
                                kind: &action.kind,
                                keys: &action.keys,
                                text: &action.text,
                                url: &action.url,
                                command: &action.command,
                                delay: &action.delay,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn deck_from_file(path: &str) -> Result<Deck, ConfigError> {
    let contents = std::fs::read_to_string(path)?;
    let file: DeckFile = toml::from_str(&contents)?;
    Ok(Deck {
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
    })
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
                    on_press: vec![ProtocolAction {
                        kind: "hotkey".to_string(),
                        keys: vec!["win".to_string(), "d".to_string()],
                        text: None,
                        url: None,
                        command: None,
                        delay: None,
                    }],
                },
                Button {
                    id: "mute".to_string(),
                    label: "Mute mic".to_string(),
                    icon: Some("fa-solid fa-microphone-slash".to_string()),
                    on_press: vec![ProtocolAction {
                        kind: "hotkey".to_string(),
                        keys: vec!["ctrl".to_string(), "shift".to_string(), "m".to_string()],
                        text: None,
                        url: None,
                        command: None,
                        delay: None,
                    }],
                },
                Button {
                    id: "web".to_string(),
                    label: "Browser".to_string(),
                    icon: Some("fa-solid fa-globe".to_string()),
                    on_press: vec![ProtocolAction {
                        kind: "url".to_string(),
                        keys: vec![],
                        text: None,
                        url: Some("https://example.com".to_string()),
                        command: None,
                        delay: None,
                    }],
                },
            ],
        }]
    }
}

pub fn find_button<'a>(deck: &'a Deck, id: &'a str) -> Option<&'a Button> {
    deck.pages.iter().flat_map(|p| p.buttons.iter()).find(|b| b.id == id)
}

#[cfg(test)]
mod tests {
    use super::{deck_default_home, deck_from_file_or_create};
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
        assert_eq!(deck.pages[0].buttons.len(), 3);
        assert_eq!(deck.pages[0].buttons[0].icon.as_deref(), Some("fa-solid fa-desktop"));
        let default_deck = deck_default_home();
        assert_eq!(deck.pages[0].buttons[1].id, default_deck.pages[0].buttons[1].id);

        fs::remove_file(path).expect("generated test deck should be removed");
    }

    #[test]
    fn keeps_existing_deck_file_unchanged() {
        let path = test_path("existing");
        let contents = "[deck]\nname = \"Custom\"\ngrid = { cols = 1, rows = 1 }\n\n[[page]]\nname = \"Home\"\n\n[[page.button]]\nid = \"custom\"\nlabel = \"Keep\"\non_press = []\n";
        fs::write(&path, contents).expect("test deck should be written");

        let deck = deck_from_file_or_create(&path).expect("existing deck should load");
        assert_eq!(deck.name, "Custom");
        assert_eq!(deck.pages[0].buttons[0].id, "custom");
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);

        fs::remove_file(path).expect("test deck should be removed");
    }
}