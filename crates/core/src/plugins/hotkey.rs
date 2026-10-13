use super::{ActionDefinition, ActionField, ActionFieldType, ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use serde_json::{Map, Value};

pub struct HotkeyPlugin;

#[async_trait]
impl ActionPlugin for HotkeyPlugin {
    fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            kind: "hotkey".to_string(),
            name: "Hotkey".to_string(),
            description: "Press a key or key combination.".to_string(),
            fields: vec![ActionField::text(
                "keys",
                "Keys (comma-separated)",
                ActionFieldType::Text,
                true,
                "ctrl, shift, m",
                "Modifiers first, followed by the key to press.",
                Some("ctrl, d"),
            )],
        }
    }

    fn validate_config(&self, config: &Map<String, Value>) -> Result<(), PluginError> {
        let keys = config["keys"]
            .as_str()
            .expect("field validation checked keys");
        if keys.split(',').any(|key| key.trim().is_empty()) {
            return Err(PluginError::InvalidConfiguration(
                "hotkey keys must not contain empty entries".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let keys: Vec<String> = config["keys"]
            .as_str()
            .expect("field validation checked keys")
            .split(',')
            .map(|key| key.trim().to_string())
            .collect();
        let (key, modifiers) = keys.split_last().expect("field validation checked keys");
        let mut held: Vec<&String> = Vec::new();
        for modifier in modifiers {
            if let Err(error) = input.key_down(modifier).await {
                for modifier in held.iter().rev() {
                    let _ = input.key_up(modifier).await;
                }
                return Err(PluginError::Input(error.to_string()));
            }
            held.push(modifier);
        }

        let click_result = input.key_click(key).await;
        let mut release_error = None;
        for modifier in held.iter().rev() {
            if let Err(error) = input.key_up(modifier).await {
                if release_error.is_none() {
                    release_error = Some(error.to_string());
                }
            }
        }
        if let Err(error) = click_result {
            return Err(PluginError::Input(error.to_string()));
        }
        if let Some(error) = release_error {
            return Err(PluginError::Input(error));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionPlugin, HotkeyPlugin};
    use crate::input::{FakeEventKind, FakeInput};
    use deskdeck_protocol::deck::Action;
    use serde_json::json;

    #[tokio::test]
    async fn holds_modifiers_while_clicking_the_final_key() {
        let plugin = HotkeyPlugin;
        let input = FakeInput::new();
        let action = Action {
            kind: "hotkey".to_string(),
            config: serde_json::from_value(json!({"keys": "ctrl, shift, m"})).unwrap(),
        };

        plugin.execute(&action.config, &input).await.unwrap();
        let events = input.events();
        assert!(matches!(events[0].kind, FakeEventKind::KeyDown));
        assert_eq!(events[0].key, "ctrl");
        assert!(matches!(events[1].kind, FakeEventKind::KeyDown));
        assert_eq!(events[1].key, "shift");
        assert!(matches!(events[2].kind, FakeEventKind::KeyDown));
        assert_eq!(events[2].key, "m");
        assert!(matches!(events[3].kind, FakeEventKind::KeyUp));
        assert_eq!(events[3].key, "m");
        assert!(matches!(events[4].kind, FakeEventKind::KeyUp));
        assert_eq!(events[4].key, "shift");
        assert!(matches!(events[5].kind, FakeEventKind::KeyUp));
        assert_eq!(events[5].key, "ctrl");
    }
}
