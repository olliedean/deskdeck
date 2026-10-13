use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub name: String,
    pub grid: Grid,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cols: u8,
    pub rows: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub name: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u8>,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub custom_css: Option<String>,
    #[serde(default)]
    pub custom_html: Option<String>,
    pub on_press: Vec<Action>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    #[serde(rename = "action")]
    pub kind: String,
    pub config: Map<String, Value>,
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ActionInput {
            #[serde(rename = "action")]
            kind: String,
            #[serde(default)]
            config: Map<String, Value>,
            #[serde(default)]
            keys: Option<Vec<String>>,
            text: Option<String>,
            url: Option<String>,
            command: Option<String>,
            delay: Option<f64>,
        }

        let mut input = ActionInput::deserialize(deserializer)?;
        if let Some(keys) = input.keys {
            input
                .config
                .entry("keys".to_string())
                .or_insert_with(|| Value::String(keys.join(", ")));
        }
        for (key, value) in [
            ("text", input.text.map(Value::String)),
            ("url", input.url.map(Value::String)),
            ("command", input.command.map(Value::String)),
            (
                "delay",
                input
                    .delay
                    .and_then(serde_json::Number::from_f64)
                    .map(Value::Number),
            ),
        ] {
            if let Some(value) = value {
                input.config.entry(key.to_string()).or_insert(value);
            }
        }

        Ok(Self {
            kind: input.kind,
            config: input.config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Action;
    use serde_json::json;

    #[test]
    fn accepts_legacy_flat_action_fields_and_serializes_config() {
        let action: Action = serde_json::from_value(json!({
            "action": "hotkey",
            "keys": ["ctrl", "d"],
            "text": "hello",
            "url": "https://example.com",
            "command": "echo hello",
            "delay": 0.5
        }))
        .unwrap();

        assert_eq!(action.config["keys"], json!("ctrl, d"));
        assert_eq!(action.config["text"], json!("hello"));
        assert_eq!(action.config["url"], json!("https://example.com"));
        assert_eq!(action.config["command"], json!("echo hello"));
        assert_eq!(action.config["delay"], json!(0.5));
        assert_eq!(
            serde_json::to_value(action).unwrap(),
            json!({
                "action": "hotkey",
                "config": {
                    "keys": "ctrl, d",
                    "text": "hello",
                    "url": "https://example.com",
                    "command": "echo hello",
                    "delay": 0.5
                }
            })
        );
    }
}
