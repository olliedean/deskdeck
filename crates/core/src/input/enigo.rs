use crate::input::{InputBackend, InputError};
use async_trait::async_trait;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::sync::Mutex;

pub struct EnigoInput {
    enigo: Mutex<Enigo>,
}

impl EnigoInput {
    pub fn new() -> Result<Self, InputError> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|error| InputError::Key(format!("initialize input backend: {error}")))?;
        Ok(EnigoInput {
            enigo: Mutex::new(enigo),
        })
    }

    fn translate_key(key: &str) -> Result<Key, InputError> {
        let normalized = key.to_lowercase();
        let key = match normalized.as_str() {
            "ctrl" | "control" => Key::Control,
            "shift" => Key::Shift,
            "alt" => Key::Alt,
            "win" | "super" | "meta" => Key::Meta,
            "up" | "uparrow" => Key::UpArrow,
            "down" | "downarrow" => Key::DownArrow,
            "left" | "leftarrow" => Key::LeftArrow,
            "right" | "rightarrow" => Key::RightArrow,
            "enter" | "return" => Key::Return,
            "esc" | "escape" => Key::Escape,
            "tab" => Key::Tab,
            "space" => Key::Space,
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" => Key::PageUp,
            "pagedown" => Key::PageDown,
            "capslock" => Key::CapsLock,
            "f1" => Key::F1,
            "f2" => Key::F2,
            "f3" => Key::F3,
            "f4" => Key::F4,
            "f5" => Key::F5,
            "f6" => Key::F6,
            "f7" => Key::F7,
            "f8" => Key::F8,
            "f9" => Key::F9,
            "f10" => Key::F10,
            "f11" => Key::F11,
            "f12" => Key::F12,
            _ => {
                let mut chars = normalized.chars();
                let character = chars
                    .next()
                    .filter(|_| chars.next().is_none())
                    .ok_or_else(|| InputError::Key(format!("unsupported key name: {key}")))?;
                Key::Unicode(character)
            }
        };
        Ok(key)
    }
}

#[async_trait]
impl InputBackend for EnigoInput {
    async fn key_down(&self, key: &str) -> Result<(), InputError> {
        let key = Self::translate_key(key)?;
        self.enigo
            .lock()
            .map_err(|_| InputError::Key("input backend lock poisoned".to_string()))?
            .key(key, Direction::Press)
            .map_err(|error| InputError::Key(format!("key down: {error}")))
    }

    async fn key_up(&self, key: &str) -> Result<(), InputError> {
        let key = Self::translate_key(key)?;
        self.enigo
            .lock()
            .map_err(|_| InputError::Key("input backend lock poisoned".to_string()))?
            .key(key, Direction::Release)
            .map_err(|error| InputError::Key(format!("key up: {error}")))
    }

    async fn key_click(&self, key: &str) -> Result<(), InputError> {
        let key = Self::translate_key(key)?;
        self.enigo
            .lock()
            .map_err(|_| InputError::Key("input backend lock poisoned".to_string()))?
            .key(key, Direction::Click)
            .map_err(|error| InputError::Key(format!("key click: {error}")))
    }

    async fn type_text(&self, text: &str) -> Result<(), InputError> {
        self.enigo
            .lock()
            .map_err(|_| InputError::Key("input backend lock poisoned".to_string()))?
            .text(text)
            .map_err(|error| InputError::Key(format!("type text: {error}")))
    }
}
