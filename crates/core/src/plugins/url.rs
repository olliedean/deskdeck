use super::{ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;

pub struct UrlPlugin;

#[async_trait]
impl ActionPlugin for UrlPlugin {
    fn kind(&self) -> &'static str {
        "url"
    }

    async fn execute(&self, action: &Action, _input: &dyn InputBackend) -> Result<(), PluginError> {
        let url = action
            .url
            .as_deref()
            .filter(|url| !url.trim().is_empty())
            .ok_or_else(|| {
                PluginError::InvalidConfiguration("url action requires a URL".to_string())
            })?;
        open::that(url).map_err(|error| PluginError::Operation(format!("open URL: {error}")))
    }
}
