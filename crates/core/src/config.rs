use deskdeck_protocol::deck::{Action as ProtocolAction, Button, Deck, Grid, Page};
use std::collections::HashMap;
use thiserror::Error;

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
    toml::from_str(&contents).map_err(Into::into)
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
                    icon: None,
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
                    icon: None,
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
                    icon: None,
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