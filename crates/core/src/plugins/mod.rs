mod command;
mod delay;
mod hotkey;
mod text;
mod url;

use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

pub use command::CommandPlugin;
pub use delay::DelayPlugin;
pub use hotkey::HotkeyPlugin;
pub use text::TypeTextPlugin;
pub use url::UrlPlugin;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid action configuration: {0}")]
    InvalidConfiguration(String),
    #[error("input operation failed: {0}")]
    Input(String),
    #[error("operation failed: {0}")]
    Operation(String),
}

#[async_trait]
pub trait ActionPlugin: Send + Sync {
    fn kind(&self) -> &'static str;

    async fn execute(&self, action: &Action, input: &dyn InputBackend) -> Result<(), PluginError>;
}

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn ActionPlugin>>,
}

impl PluginRegistry {
    /// Creates a registry with the built-in DeskDeck actions.
    pub fn with_defaults() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
        };
        registry.register(Arc::new(HotkeyPlugin));
        registry.register(Arc::new(UrlPlugin));
        registry.register(Arc::new(CommandPlugin));
        registry.register(Arc::new(TypeTextPlugin));
        registry.register(Arc::new(DelayPlugin));
        registry
    }

    pub fn register(&mut self, plugin: Arc<dyn ActionPlugin>) {
        self.plugins.insert(plugin.kind().to_string(), plugin);
    }

    pub async fn execute(
        &self,
        action: &Action,
        input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let plugin = self.plugins.get(&action.kind).ok_or_else(|| {
            PluginError::InvalidConfiguration(format!(
                "no action plugin is registered for {:?}",
                action.kind
            ))
        })?;
        plugin.execute(action, input).await
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionPlugin, PluginError, PluginRegistry};
    use crate::input::{FakeEventKind, FakeInput, InputBackend};
    use async_trait::async_trait;
    use deskdeck_protocol::deck::Action;
    use std::sync::Arc;

    fn action(kind: &str) -> Action {
        Action {
            kind: kind.to_string(),
            keys: vec!["ctrl".to_string(), "d".to_string()],
            text: Some("hello".to_string()),
            url: None,
            command: None,
            delay: None,
        }
    }

    struct CustomPlugin;

    #[async_trait]
    impl ActionPlugin for CustomPlugin {
        fn kind(&self) -> &'static str {
            "custom"
        }

        async fn execute(
            &self,
            _action: &Action,
            input: &dyn InputBackend,
        ) -> Result<(), PluginError> {
            input
                .type_text("plugin")
                .await
                .map_err(|error| PluginError::Input(error.to_string()))
        }
    }

    #[tokio::test]
    async fn default_registry_runs_hotkey_and_type_plugins() {
        let registry = PluginRegistry::with_defaults();
        let input = FakeInput::new();

        registry.execute(&action("hotkey"), &input).await.unwrap();
        registry.execute(&action("type"), &input).await.unwrap();

        let events = input.events();
        assert!(matches!(events[0].kind, FakeEventKind::KeyDown));
        assert!(matches!(events[2].kind, FakeEventKind::KeyDown));
        assert!(matches!(events[4].kind, FakeEventKind::Type));
        assert_eq!(events[4].key, "hello");
    }

    #[tokio::test]
    async fn custom_plugin_can_be_registered() {
        let mut registry = PluginRegistry::with_defaults();
        registry.register(Arc::new(CustomPlugin));
        let input = FakeInput::new();

        registry.execute(&action("custom"), &input).await.unwrap();

        assert_eq!(input.events()[0].key, "plugin");
    }

    #[tokio::test]
    async fn unknown_plugin_is_reported() {
        let registry = PluginRegistry::with_defaults();
        let error = registry
            .execute(&action("missing"), &FakeInput::new())
            .await;

        assert!(matches!(error, Err(PluginError::InvalidConfiguration(_))));
    }
}
