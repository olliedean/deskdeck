use deskdeck_protocol::client::HostMessage;
use deskdeck_protocol::deck::Deck;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::find_button;
use crate::input::InputBackend;
use crate::plugins::{PluginError, PluginRegistry};

pub struct Engine {
    deck: Arc<RwLock<Deck>>,
    input: Arc<dyn InputBackend>,
    plugins: PluginRegistry,
}

#[derive(Debug, Clone)]
pub enum EngineError {
    ButtonNotFound(String),
    Input(String),
    Plugin(String),
}

impl From<crate::input::InputError> for EngineError {
    fn from(e: crate::input::InputError) -> Self {
        EngineError::Input(e.to_string())
    }
}

impl From<PluginError> for EngineError {
    fn from(error: PluginError) -> Self {
        match error {
            PluginError::Input(message) => EngineError::Input(message),
            other => EngineError::Plugin(other.to_string()),
        }
    }
}

impl Engine {
    pub fn new(deck: Deck, input: Arc<dyn InputBackend>) -> Self {
        Self::with_plugins(deck, input, PluginRegistry::with_defaults())
    }

    pub fn with_plugins(deck: Deck, input: Arc<dyn InputBackend>, plugins: PluginRegistry) -> Self {
        Engine {
            deck: Arc::new(RwLock::new(deck)),
            input,
            plugins,
        }
    }

    pub async fn deck(&self) -> Deck {
        self.deck.read().await.clone()
    }

    pub async fn set_deck(&self, deck: Deck) {
        *self.deck.write().await = deck;
    }

    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    pub async fn press(
        &self,
        button_id: &str,
        down: bool,
    ) -> Result<Vec<HostMessage>, EngineError> {
        if !down {
            return Ok(vec![]);
        }

        let actions = {
            let deck = self.deck.read().await;
            find_button(&deck, button_id)
                .map(|button| button.on_press.clone())
                .ok_or_else(|| EngineError::ButtonNotFound(button_id.to_string()))?
        };

        for action in &actions {
            self.plugins.execute(action, self.input.as_ref()).await?;
        }
        Ok(vec![])
    }
}
