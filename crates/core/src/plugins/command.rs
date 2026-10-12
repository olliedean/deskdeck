use super::{ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;
use tokio::process::Command;

pub struct CommandPlugin;

#[async_trait]
impl ActionPlugin for CommandPlugin {
    fn kind(&self) -> &'static str {
        "command"
    }

    async fn execute(&self, action: &Action, _input: &dyn InputBackend) -> Result<(), PluginError> {
        let command = action
            .command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
            .ok_or_else(|| {
                PluginError::InvalidConfiguration("command action requires a command".to_string())
            })?;
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
            .map_err(|error| PluginError::Operation(format!("start command: {error}")))?;

        tokio::spawn(async move {
            if let Err(error) = child.wait().await {
                eprintln!("DeskDeck command plugin failed waiting for child: {error}");
            }
        });
        Ok(())
    }
}
