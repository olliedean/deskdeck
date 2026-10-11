use deskdeck_protocol::deck::Deck;
use deskdeck_protocol::client::{ClientMessage, HostMessage};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::Command;

use crate::config::{deck_default_home, find_button};
use crate::input::InputBackend;

pub struct Engine {
    deck: Arc<RwLock<Deck>>,
    input: Arc<dyn InputBackend>,
}

#[derive(Debug, Clone)]
pub enum EngineError {
    ButtonNotFound(String),
    Input(String),
}

impl From<crate::input::InputError> for EngineError {
    fn from(e: crate::input::InputError) -> Self {
        EngineError::Input(e.to_string())
    }
}

impl Engine {
    pub fn new(deck: Deck, input: Arc<dyn InputBackend>) -> Self {
        Engine { deck: Arc::new(RwLock::new(deck)), input }
    }

    pub async fn deck(&self) -> Deck {
        self.deck.read().await.clone()
    }

    pub async fn set_deck(&self, deck: Deck) {
        *self.deck.write().await = deck;
    }

    pub async fn press(&self, button_id: &str, down: bool) -> Result<Vec<HostMessage>, EngineError> {
        let deck = self.deck.read().await;
        let button = find_button(&deck, button_id).ok_or_else(|| EngineError::ButtonNotFound(button_id.to_string()))?;

        if !down {
            return Ok(vec![]);
        }

        let mut patches = Vec::new();
        for action in &button.on_press {
            self.run_action(action).await.map(|patch| patches.extend(patch))?;
        }
        Ok(patches)
    }

    async fn run_action(&self, action: &deskdeck_protocol::deck::Action) -> Result<Vec<HostMessage>, EngineError> {
        match action.kind.as_str() {
            "hotkey" => {
                for k in &action.keys {
                    self.input.key_click(k).await?;
                }
                Ok(vec![])
            }
            "url" => {
                if let Some(url) = &action.url {
                    let _ = open::that(url);
                    Ok(vec![])
                } else {
                    Err(EngineError::Input("url action missing URL".to_string()))
                }
            }
            "command" => {
                if let Some(cmd) = &action.command {
                    let c = cmd.clone();
                    tokio::spawn(async move {
                        let _ = Command::new("sh").arg("-c").arg(&c).status().await;
                    });
                    Ok(vec![])
                } else {
                    Err(EngineError::Input("command action missing command".to_string()))
                }
            }
            "type" => {
                if let Some(text) = &action.text {
                    self.input.type_text(text).await?;
                    Ok(vec![])
                } else {
                    Err(EngineError::Input("type action missing text".to_string()))
                }
            }
            "delay" => {
                if let Some(secs) = action.delay {
                    tokio::time::sleep(std::time::Duration::from_secs_f64(secs)).await;
                }
                Ok(vec![])
            }
            _ => Ok(vec![])
        }
    }
}