use super::{ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;

pub struct HotkeyPlugin;

#[async_trait]
impl ActionPlugin for HotkeyPlugin {
    fn kind(&self) -> &'static str {
        "hotkey"
    }

    async fn execute(&self, action: &Action, input: &dyn InputBackend) -> Result<(), PluginError> {
        if action.keys.is_empty() {
            return Err(PluginError::InvalidConfiguration(
                "hotkey action requires at least one key".to_string(),
            ));
        }
        for key in &action.keys {
            input
                .key_click(key)
                .await
                .map_err(|error| PluginError::Input(error.to_string()))?;
        }
        Ok(())
    }
}
