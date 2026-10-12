use deskdeck_protocol::deck::{Action as ProtocolAction, Button, Deck, Grid, Page};
use serde::Deserialize;
use thiserror::Error;

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
    use super::deck_from_file;

    #[test]
    fn loads_buttons_and_icons_from_deck_toml() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../deck.toml");
        let deck = deck_from_file(path).expect("deck.toml should parse");
        let buttons = &deck.pages[0].buttons;

        assert_eq!(buttons.len(), 3);
        assert_eq!(buttons[0].icon.as_deref(), Some("fa-solid fa-desktop"));
        assert_eq!(buttons[1].icon.as_deref(), Some("fa-solid fa-microphone-slash"));
        assert_eq!(buttons[2].icon.as_deref(), Some("fa-solid fa-globe"));
    }
}