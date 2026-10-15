use super::{ActionDefinition, ActionField, ActionFieldType, ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::process::Command;

pub struct CommandPlugin;

#[async_trait]
impl ActionPlugin for CommandPlugin {
    fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            kind: "command".to_string(),
            name: "Run command".to_string(),
            description: "Run a shell command on the DeskDeck host.".to_string(),
            fields: vec![ActionField::text(
                "command",
                "Command",
                ActionFieldType::Multiline,
                true,
                "your-command",
                "Runs with the host user's permissions.",
                None,
            )],
        }
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        _input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let command = config["command"]
            .as_str()
            .expect("field validation checked command");
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
            .map_err(|error| PluginError::Operation(format!("start command: {error}")))?;

        tokio::spawn(async move {
            if let Err(error) = child.wait() {
                eprintln!("DeskDeck command plugin failed waiting for child: {error}");
            }
        });
        Ok(())
    }
}
