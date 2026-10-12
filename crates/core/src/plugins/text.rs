use super::{ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;

pub struct TypeTextPlugin;

#[async_trait]
impl ActionPlugin for TypeTextPlugin {
    fn kind(&self) -> &'static str {
        "type"
    }

    async fn execute(&self, action: &Action, input: &dyn InputBackend) -> Result<(), PluginError> {
        let text = action.text.as_deref().ok_or_else(|| {
            PluginError::InvalidConfiguration("type action requires text".to_string())
        })?;
        input
            .type_text(text)
            .await
            .map_err(|error| PluginError::Input(error.to_string()))
    }
}
