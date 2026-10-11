use serde::{Deserialize, Serialize};
use crate::deck::Deck;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: u32,
    pub token: String,
    pub screen_width: u32,
    pub screen_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Press {
    pub button_id: String,
    pub down: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetValue {
    pub control_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Hello(Hello),
    Press(Press),
    SetValue(SetValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    pub host_version: String,
    pub deck: Deck,
    pub button_states: Vec<ButtonState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonState {
    pub button_id: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatePatch {
    ButtonActive { button_id: String, active: bool },
    ButtonLabel { button_id: String, label: String },
    ButtonIcon { button_id: String, icon: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostMessage {
    Welcome(Welcome),
    DeckUpdated(Deck),
    StatePatch(StatePatch),
    Error(HostError),
}