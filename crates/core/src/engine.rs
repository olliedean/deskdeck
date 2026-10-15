use deskdeck_protocol::client::HostMessage;
use deskdeck_protocol::deck::{Action as ProtocolAction, Deck as ProtocolDeck};
use std::sync::Arc;
use std::sync::RwLock;

use crate::domain::{ButtonId, ButtonState, DomainError, Profile, Slot};
use crate::input::InputBackend;
use crate::plugins::{PluginError, PluginRegistry};

pub struct Engine {
    profile: Arc<RwLock<Profile>>,
    input: Arc<dyn InputBackend>,
    plugins: PluginRegistry,
}

#[derive(Debug, Clone)]
pub enum EngineError {
    ButtonNotFound(String),
    Input(String),
    Plugin(String),
    Domain(String),
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

impl From<DomainError> for EngineError {
    fn from(error: DomainError) -> Self {
        EngineError::Domain(error.to_string())
    }
}

impl Engine {
    pub fn new(profile: Profile, input: Arc<dyn InputBackend>) -> Self {
        Self::with_plugins(profile, input, PluginRegistry::with_defaults())
    }

    pub fn with_plugins(
        profile: Profile,
        input: Arc<dyn InputBackend>,
        plugins: PluginRegistry,
    ) -> Self {
        Engine {
            profile: Arc::new(RwLock::new(profile)),
            input,
            plugins,
        }
    }

    pub async fn profile(&self) -> Profile {
        self.profile.read().expect("Poisoned RwLock").clone()
    }

    pub async fn deck(&self) -> ProtocolDeck {
        self.profile.read().expect("Poisoned RwLock").to_deck()
    }

    pub async fn set_profile(&self, profile: Profile) {
        *self.profile.write().expect("Poisoned RwLock") = profile;
    }

    pub async fn set_deck(&self, deck: ProtocolDeck) -> Result<(), DomainError> {
        self.set_profile(Profile::try_from(deck)?).await;
        Ok(())
    }

    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    pub async fn press(
        &self,
        button_id: &str,
        down: bool,
    ) -> Result<Vec<HostMessage>, EngineError> {
        let actions = {
            let mut profile = self.profile.write().expect("Poisoned RwLock");
            let id = ButtonId::new(button_id.to_string())?;
            let button = profile
                .pages
                .iter_mut()
                .flat_map(|page| page.slots.iter_mut())
                .find_map(|slot| match slot {
                    Slot::Button(button) if button.id == id => Some(button),
                    _ => None,
                })
                .ok_or_else(|| EngineError::ButtonNotFound(button_id.to_string()))?;
            button.state = if down {
                ButtonState::Pressed
            } else {
                ButtonState::ToggledOff
            };
            if down {
                button.actions.clone()
            } else {
                return Ok(vec![]);
            }
        };

        for action in &actions {
            let action = ProtocolAction::from(action);
            self.plugins.execute(&action, self.input.as_ref()).await?;
        }
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::Engine;
    use crate::{
        config::deck_default_home,
        domain::{ButtonState, Profile, Slot},
        input::FakeInput,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn press_updates_runtime_button_state_and_executes_actions() {
        let profile = Profile::try_from(deck_default_home()).unwrap();
        let input = Arc::new(FakeInput::new());
        let engine = Engine::new(profile, input.clone());

        engine.press("home", true).await.unwrap();
        assert_eq!(input.events().len(), 4);
        assert!(matches!(
            &engine.profile().await.pages[0].slots[0],
            Slot::Button(button) if button.state == ButtonState::Pressed
        ));

        engine.press("home", false).await.unwrap();
        assert!(matches!(
            &engine.profile().await.pages[0].slots[0],
            Slot::Button(button) if button.state == ButtonState::ToggledOff
        ));
    }
}
