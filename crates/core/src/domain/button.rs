use super::ButtonId;
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct ActionInstance {
    pub kind: String,
    pub config: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ButtonAppearance {
    pub icon: Option<String>,
    pub background_image: Option<String>,
    pub custom_css: Option<String>,
    pub custom_html: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonState {
    #[default]
    Idle,
    Active,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub id: ButtonId,
    pub label: String,
    pub appearance: ButtonAppearance,
    pub actions: Vec<ActionInstance>,
    pub state: ButtonState,
}
