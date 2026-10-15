use super::{ButtonId, DomainError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::time::SystemTime;
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
    pub enabled: bool,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
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

impl Button {
    pub fn new(id: ButtonId, label: String) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            label,
            appearance: ButtonAppearance::default(),
            content: ButtonContent::default(),
            actions: Vec::new(),
            state: ButtonState::Default,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = SystemTime::now();
    }

    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = SystemTime::now();
    }

    pub fn update_appearance(&mut self, appearance: ButtonAppearance) {
        self.appearance = appearance;
        self.updated_at = SystemTime::now();
    }

    pub fn update_content(&mut self, content: ButtonContent) {
        self.content = content;
        self.updated_at = SystemTime::now();
    }

    pub fn update_state(&mut self, state: ButtonState) {
        self.state = state;
        self.updated_at = SystemTime::now();
    }

    pub fn add_action(&mut self, action: ActionInstance) -> Result<(), DomainError> {
        if self.actions.iter().any(|a| a.id == action.id) {
            return Err(DomainError::InvalidId { kind: "action" });
        }
        self.actions.push(action);
        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub fn remove_action(&mut self, id: &ActionInstanceId) -> Result<(), DomainError> {
        let initial_len = self.actions.len();
        self.actions.retain(|a| &a.id != id);
        if self.actions.len() == initial_len {
            return Err(DomainError::InvalidId { kind: "action" });
        }
        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub fn reorder_actions(&mut self, ids: &[ActionInstanceId]) -> Result<(), DomainError> {
        for id in ids {
            if !self.actions.iter().any(|a| &a.id == id) {
                return Err(DomainError::InvalidId { kind: "action" });
            }
        }

        if ids.len() != self.actions.len() {
            return Err(DomainError::InvalidId { kind: "action" });
        }

        let mut reordered = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(pos) = self.actions.iter().position(|a| &a.id == id) {
                reordered.push(self.actions.remove(pos));
            }
        }
        self.actions = reordered;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub fn enable_action(&mut self, id: &ActionInstanceId) -> Result<(), DomainError> {
        if let Some(action) = self.actions.iter_mut().find(|a| &a.id == id) {
            action.enabled = true;
            self.updated_at = SystemTime::now();
            Ok(())
        } else {
            Err(DomainError::InvalidId { kind: "action" })
        }
    }

    pub fn disable_action(&mut self, id: &ActionInstanceId) -> Result<(), DomainError> {
        if let Some(action) = self.actions.iter_mut().find(|a| &a.id == id) {
            action.enabled = false;
            self.updated_at = SystemTime::now();
            Ok(())
        } else {
            Err(DomainError::InvalidId { kind: "action" })
        }
    }
}

impl Serialize for Button {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Button", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("label", &self.label)?;
        state.serialize_field("appearance", &self.appearance)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("actions", &self.actions)?;
        state.serialize_field("state", &self.state)?;
        state.serialize_field("enabled", &self.enabled)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Button {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            id,
            label,
            appearance,
            content,
            actions,
            state,
            enabled,
            created_at,
            updated_at,
        }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`id`, `label`, `appearance`, `content`, `actions`, `state`, `enabled`, `created_at`, or `updated_at`")
                    }
                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(Field::id),
                            "label" => Ok(Field::label),
                            "appearance" => Ok(Field::appearance),
                            "content" => Ok(Field::content),
                            "actions" => Ok(Field::actions),
                            "state" => Ok(Field::state),
                            "enabled" => Ok(Field::enabled),
                            "created_at" => Ok(Field::created_at),
                            "updated_at" => Ok(Field::updated_at),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        struct ButtonVisitor;
        impl<'de> serde::de::Visitor<'de> for ButtonVisitor {
            type Value = Button;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Button")
            }
            fn visit_map<V>(self, mut map: V) -> Result<Button, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut id: Option<ButtonId> = None;
                let mut label: Option<String> = None;
                let mut appearance: Option<ButtonAppearance> = None;
                let mut content: Option<ButtonContent> = None;
                let mut actions: Option<Vec<ActionInstance>> = None;
                let mut state: Option<ButtonState> = None;
                let mut enabled: Option<bool> = None;
                let mut created_at: Option<SystemTime> = None;
                let mut updated_at: Option<SystemTime> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::label => {
                            if label.is_some() {
                                return Err(serde::de::Error::duplicate_field("label"));
                            }
                            label = Some(map.next_value()?);
                        }
                        Field::appearance => {
                            if appearance.is_some() {
                                return Err(serde::de::Error::duplicate_field("appearance"));
                            }
                            appearance = Some(map.next_value()?);
                        }
                        Field::content => {
                            if content.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content = Some(map.next_value()?);
                        }
                        Field::actions => {
                            if actions.is_some() {
                                return Err(serde::de::Error::duplicate_field("actions"));
                            }
                            actions = Some(map.next_value()?);
                        }
                        Field::state => {
                            if state.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state = Some(map.next_value()?);
                        }
                        Field::enabled => {
                            if enabled.is_some() {
                                return Err(serde::de::Error::duplicate_field("enabled"));
                            }
                            enabled = Some(map.next_value()?);
                        }
                        Field::created_at => {
                            if created_at.is_some() {
                                return Err(serde::de::Error::duplicate_field("created_at"));
                            }
                            created_at = Some(map.next_value()?);
                        }
                        Field::updated_at => {
                            if updated_at.is_some() {
                                return Err(serde::de::Error::duplicate_field("updated_at"));
                            }
                            updated_at = Some(map.next_value()?);
                        }
                    }
                }
                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let label = label.ok_or_else(|| serde::de::Error::missing_field("label"))?;
                let appearance =
                    appearance.ok_or_else(|| serde::de::Error::missing_field("appearance"))?;
                let content = content.ok_or_else(|| serde::de::Error::missing_field("content"))?;
                let actions = actions.unwrap_or(Vec::new());
                let state = state.unwrap_or(ButtonState::Default);
                let enabled = enabled.unwrap_or(true);
                let created_at = created_at.unwrap_or(SystemTime::now());
                let updated_at = updated_at.unwrap_or(SystemTime::now());
                Ok(Button {
                    id,
                    label,
                    appearance,
                    content,
                    actions,
                    state,
                    enabled,
                    created_at,
                    updated_at,
                })
            }
        }
        const FIELDS: &[&str] = &[
            "id",
            "label",
            "appearance",
            "content",
            "actions",
            "state",
            "enabled",
            "created_at",
            "updated_at",
        ];
        deserializer.deserialize_struct("Button", FIELDS, ButtonVisitor)
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
