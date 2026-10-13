use super::{ActionDefinition, ActionField, ActionFieldType, ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use serde_json::{Map, Value};

pub struct TypeTextPlugin;

#[async_trait]
impl ActionPlugin for TypeTextPlugin {
    fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            kind: "type".to_string(),
            name: "Type text".to_string(),
            description: "Type text into the currently focused application.".to_string(),
            fields: vec![ActionField::text(
                "text",
                "Text",
                ActionFieldType::Multiline,
                true,
                "Text to type",
                "Text is sent as keyboard input to the active application.",
                None,
            )],
        }
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let text = config["text"]
            .as_str()
            .expect("field validation checked text");
        input
            .type_text(text)
            .await
            .map_err(|error| PluginError::Input(error.to_string()))
    }
}
