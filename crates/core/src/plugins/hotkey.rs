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
        let (key, modifiers) = action.keys.split_last().expect("keys were checked");
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

    #[tokio::test]
    async fn holds_modifiers_while_clicking_the_final_key() {
        let plugin = HotkeyPlugin;
        let input = FakeInput::new();
        let action = Action {
            kind: "hotkey".to_string(),
            keys: vec!["ctrl".to_string(), "shift".to_string(), "m".to_string()],
            text: None,
            url: None,
            command: None,
            delay: None,
        };

        plugin.execute(&action, &input).await.unwrap();
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
