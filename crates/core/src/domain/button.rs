use super::{ButtonId, DomainError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonAppearance {
    pub label: Option<String>,
    pub icon: Option<String>,
    pub background_color: Option<String>,
    pub text_color: Option<String>,
    pub font_size: Option<u8>,
    pub horizontal_alignment: Option<HorizontalAlignment>,
    pub vertical_alignment: Option<VerticalAlignment>,
    pub padding: Option<u8>,
    pub corner_radius: Option<u8>,
    pub border: Option<Border>,
    pub opacity: Option<u8>,
    pub badge_text: Option<String>,
    pub badge_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerticalAlignment {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Border {
    pub width: u8,
    pub color: String,
    pub style: BorderStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ButtonState {
    #[default]
    Default,
    Pressed,
    ToggledOn,
    ToggledOff,
    Loading,
    Error,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ButtonContent {
    Basic {
        label: Option<String>,
        icon: Option<String>,
    },
    Custom {
        markup: String,
        css: Option<String>,
    },
}

impl Default for ButtonContent {
    fn default() -> Self {
        ButtonContent::Basic {
            label: None,
            icon: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub id: ButtonId,
    pub label: String,
    pub appearance: ButtonAppearance,
    pub content: ButtonContent,
    pub actions: Vec<ActionInstance>,
    pub state: ButtonState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInstanceId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInstance {
    pub id: ActionInstanceId,
    pub action_type: String,
    pub settings: Value,
    pub enabled: bool,
}

impl ActionInstance {
    pub fn new(action_type: impl Into<String>, settings: Value) -> Result<Self, DomainError> {
        let action_type = action_type.into().trim().to_string();
        if action_type.is_empty() {
            return Err(DomainError::EmptyName {
                kind: "action_type",
            });
        }

        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
        let name = action_type.as_bytes();
        let mut hasher = Sha1::new();
        hasher.update(namespace.as_bytes());
        hasher.update(name);
        let hash = hasher.finalize();
        let mut id_bytes = [0u8; 16];
        id_bytes.copy_from_slice(&hash[..16]);
        id_bytes[6] = (id_bytes[6] & 0x0f) | 0x50;
        id_bytes[8] = (id_bytes[8] & 0x3f) | 0x80;
        let id = Uuid::from_bytes(id_bytes);

        Ok(Self {
            id: ActionInstanceId(id),
            action_type,
            settings,
            enabled: true,
        })
    }
}

impl ButtonContent {
    pub fn new_basic() -> Self {
        ButtonContent::Basic {
            label: None,
            icon: None,
        }
    }

    pub fn new_custom(markup: String, css: Option<String>) -> Result<Self, DomainError> {
        const CONTENT_SIZE_LIMIT: usize = 10240;

        let markup_size = markup.len();
        let css_size = css.as_ref().map_or(0, |s| s.len());
        let total_size = markup_size + css_size;

        if total_size > CONTENT_SIZE_LIMIT {
            return Err(DomainError::ContentSizeExceeded {
                limit: CONTENT_SIZE_LIMIT,
            });
        }

        Ok(ButtonContent::Custom { markup, css })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_basic_content() {
        let content = ButtonContent::default();
        assert!(matches!(
            content,
            ButtonContent::Basic {
                label: None,
                icon: None
            }
        ));
    }

    #[test]
    fn creating_custom_content() {
        let markup = "<div>Hello World</div>".to_string();
        let css = Some("color: red;".to_string());
        let content = ButtonContent::new_custom(markup.clone(), css.clone()).unwrap();

        assert!(
            matches!(content, ButtonContent::Custom { markup: m, css: c } if m == markup && c == css)
        );
    }

    #[test]
    fn rejecting_oversized_markup() {
        let oversized_markup = "x".repeat(10241);
        let result = ButtonContent::new_custom(oversized_markup, None);
        assert!(matches!(
            result,
            Err(DomainError::ContentSizeExceeded { limit: 10240 })
        ));
    }

    #[test]
    fn rejecting_oversized_css() {
        let oversized_css = "x".repeat(10241);
        let result = ButtonContent::new_custom("".to_string(), Some(oversized_css));
        assert!(matches!(
            result,
            Err(DomainError::ContentSizeExceeded { limit: 10240 })
        ));
    }

    #[test]
    fn serialisation_round_trips() {
        let original = ButtonContent::Basic {
            label: Some("Test".to_string()),
            icon: Some("icon-test".to_string()),
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ButtonContent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let original = ButtonContent::Custom {
            markup: "<div>Test</div>".to_string(),
            css: Some("color: blue;".to_string()),
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ButtonContent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn button_state_defaults() {
        assert_eq!(ButtonState::default(), ButtonState::Default);
    }

    #[test]
    fn button_appearance_defaults() {
        let appearance = ButtonAppearance::default();
        assert_eq!(appearance.label, None);
        assert_eq!(appearance.icon, None);
        assert_eq!(appearance.background_color, None);
        assert_eq!(appearance.text_color, None);
        assert_eq!(appearance.font_size, None);
        assert_eq!(appearance.horizontal_alignment, None);
        assert_eq!(appearance.vertical_alignment, None);
        assert_eq!(appearance.padding, None);
        assert_eq!(appearance.corner_radius, None);
        assert_eq!(appearance.border, None);
        assert_eq!(appearance.opacity, None);
        assert_eq!(appearance.badge_text, None);
        assert_eq!(appearance.badge_color, None);
    }

    #[test]
    fn creating_action_instance() {
        let action = ActionInstance::new("test_action", json!({ "param": "value" })).unwrap();
        assert_eq!(action.action_type, "test_action");
        assert_eq!(action.settings, json!({ "param": "value" }));
        assert!(action.enabled);
    }

    #[test]
    fn rejecting_empty_action_type() {
        let result = ActionInstance::new("", json!({}));
        assert!(matches!(
            result,
            Err(DomainError::EmptyName {
                kind: "action_type"
            })
        ));

        let result = ActionInstance::new("   ", json!({}));
        assert!(matches!(
            result,
            Err(DomainError::EmptyName {
                kind: "action_type"
            })
        ));

        let result = ActionInstance::new("\t\n  ", json!({}));
        assert!(matches!(
            result,
            Err(DomainError::EmptyName {
                kind: "action_type"
            })
        ));
    }

    #[test]
    fn action_instance_serialisation_round_trips() {
        let original = ActionInstance::new("test_action", json!({ "key": "value" })).unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ActionInstance = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);

        let original =
            ActionInstance::new("another_action", json!({ "numbers": [1, 2, 3] })).unwrap();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: ActionInstance = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn action_instance_defaults_enabled() {
        let action = ActionInstance::new("test_action", json!({})).unwrap();
        assert!(action.enabled);
    }
}
