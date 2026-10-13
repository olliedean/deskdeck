use super::{ActionDefinition, ActionField, ActionFieldType, ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use serde_json::{Map, Value};

pub struct UrlPlugin;

#[async_trait]
impl ActionPlugin for UrlPlugin {
    fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            kind: "url".to_string(),
            name: "Open URL".to_string(),
            description: "Open a website in the default browser.".to_string(),
            fields: vec![ActionField::text(
                "url",
                "URL",
                ActionFieldType::Text,
                true,
                "https://example.com",
                "Include http:// or https://.",
                None,
            )],
        }
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        _input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let url = config["url"]
            .as_str()
            .expect("field validation checked URL");
        open::that(url).map_err(|error| PluginError::Operation(format!("open URL: {error}")))
    }
}
