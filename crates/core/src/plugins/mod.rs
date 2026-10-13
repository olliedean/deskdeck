mod command;
mod delay;
mod hotkey;
mod text;
mod url;

use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;
use serde::Serialize;
use serde_json::{Map, Value};
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionFieldType {
    Text,
    Multiline,
    Number,
    Select,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: ActionFieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

impl ActionField {
    fn text(
        key: &str,
        label: &str,
        field_type: ActionFieldType,
        required: bool,
        placeholder: &str,
        help: &str,
        default: Option<&str>,
    ) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            field_type,
            required,
            placeholder: Some(placeholder.to_string()),
            help: Some(help.to_string()),
            default: default.map(|value| Value::String(value.to_string())),
            options: vec![],
            min: None,
            max: None,
            step: None,
        }
    }

    fn number(key: &str, label: &str, default: f64, min: f64, step: f64) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            field_type: ActionFieldType::Number,
            required: true,
            placeholder: None,
            help: None,
            default: Some(Value::from(default)),
            options: vec![],
            min: Some(min),
            max: None,
            step: Some(step),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionDefinition {
    pub kind: String,
    pub name: String,
    pub description: String,
    pub fields: Vec<ActionField>,
}

#[async_trait]
pub trait ActionPlugin: Send + Sync {
    fn definition(&self) -> ActionDefinition;

    fn validate_config(&self, _config: &Map<String, Value>) -> Result<(), PluginError> {
        Ok(())
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        input: &dyn InputBackend,
    ) -> Result<(), PluginError>;
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
        self.plugins.insert(plugin.definition().kind, plugin);
    }

    pub fn definitions(&self) -> Vec<ActionDefinition> {
        let mut definitions: Vec<_> = self
            .plugins
            .values()
            .map(|plugin| plugin.definition())
            .collect();
        definitions.sort_by(|left, right| left.kind.cmp(&right.kind));
        definitions
    }

    pub fn validate(&self, action: &Action) -> Result<(), PluginError> {
        let plugin = self.plugin(&action.kind)?;
        let definition = plugin.definition();
        for field in &definition.fields {
            let value = action.config.get(&field.key);
            if value.is_none() && field.required {
                return Err(PluginError::InvalidConfiguration(format!(
                    "{} action requires {}",
                    definition.name, field.label
                )));
            }
            let Some(value) = value else {
                continue;
            };
            let valid = match field.field_type {
                ActionFieldType::Text | ActionFieldType::Multiline => value
                    .as_str()
                    .is_some_and(|text| !field.required || !text.trim().is_empty()),
                ActionFieldType::Number => value.as_f64().is_some_and(|number| {
                    number.is_finite()
                        && field.min.is_none_or(|min| number >= min)
                        && field.max.is_none_or(|max| number <= max)
                }),
                ActionFieldType::Select => value
                    .as_str()
                    .is_some_and(|choice| field.options.iter().any(|option| option == choice)),
            };
            if !valid {
                return Err(PluginError::InvalidConfiguration(format!(
                    "{} has an invalid {} value",
                    definition.name, field.label
                )));
            }
        }
        if let Some(key) = action.config.keys().find(|key| {
            !definition
                .fields
                .iter()
                .any(|field| field.key.as_str() == key.as_str())
        }) {
            return Err(PluginError::InvalidConfiguration(format!(
                "{} does not support the {:?} setting",
                definition.name, key
            )));
        }
        plugin.validate_config(&action.config)
    }

    pub async fn execute(
        &self,
        action: &Action,
        input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        self.validate(action)?;
        self.plugin(&action.kind)?
            .execute(&action.config, input)
            .await
    }

    fn plugin(&self, kind: &str) -> Result<&Arc<dyn ActionPlugin>, PluginError> {
        self.plugins.get(kind).ok_or_else(|| {
            PluginError::InvalidConfiguration(format!(
                "no action plugin is registered for {:?}",
                kind
            ))
        })
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionDefinition, ActionPlugin, PluginError, PluginRegistry};
    use crate::input::{FakeEventKind, FakeInput, InputBackend};
    use async_trait::async_trait;
    use deskdeck_protocol::deck::Action;
    use serde_json::{Map, Value};
    use std::sync::Arc;

    fn action(kind: &str) -> Action {
        Action {
            kind: kind.to_string(),
            config: match kind {
                "hotkey" => serde_json::from_value(serde_json::json!({"keys": "ctrl, d"})),
                "type" => serde_json::from_value(serde_json::json!({"text": "hello"})),
                _ => serde_json::from_value(serde_json::json!({})),
            }
            .unwrap(),
        }
    }

    struct CustomPlugin;

    #[async_trait]
    impl ActionPlugin for CustomPlugin {
        fn definition(&self) -> ActionDefinition {
            ActionDefinition {
                kind: "custom".to_string(),
                name: "Custom".to_string(),
                description: "Test plugin".to_string(),
                fields: vec![],
            }
        }

        async fn execute(
            &self,
            _config: &Map<String, Value>,
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
        assert!(matches!(events[1].kind, FakeEventKind::KeyDown));
        assert!(matches!(events[2].kind, FakeEventKind::KeyUp));
        assert!(matches!(events[3].kind, FakeEventKind::KeyUp));
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

    #[test]
    fn exposes_plugin_defined_action_fields() {
        let definitions = PluginRegistry::with_defaults().definitions();
        let text = definitions
            .iter()
            .find(|definition| definition.kind == "type")
            .unwrap();

        assert_eq!(text.fields[0].key, "text");
        assert!(text.fields[0].required);
    }
}
