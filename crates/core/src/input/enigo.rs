use enigo::{Enigo, Key, MouseOrder};
use crate::input::{InputBackend, InputError};

pub struct EnigoInput {
    enigo: Enigo,
}

impl EnigoInput {
    pub fn new() -> Result<Self, InputError> {
        let enigo = Enigo::new().map_err(|e| InputError::Key(format!("enigo init: {}", e)))?;
        Ok(EnigoInput { enigo })
    }
}

#[async_trait]
impl InputBackend for EnigoInput {
    async fn key_down(&self, key: &str) -> Result<(), InputError> {
        let key_code = self.translate_key(key)?;
        self.enigo.key_down(key_code).map_err(|e| InputError::Key(format!("key_down: {}", e)))?;
        Ok(())
    }

    async fn key_up(&self, key: &str) -> Result<(), InputError> {
        let key_code = self.translate_key(key)?;
        self.enigo.key_up(key_code).map_err(|e| InputError::Key(format!("key_up: {}", e)))?;
        Ok(())
    }

    async fn key_click(&self, key: &str) -> Result<(), InputError> {
        self.key_down(key).await?;
        self.key_up(key).await?;
        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<(), InputError> {
        for ch in text.chars() {
            let key_code = self.char_to_key(ch)?;
            self.enigo.key_click(key_code).map_err(|e| InputError::Key(format!("type char {}: {}", ch, e)))?;
        }
        Ok(())
    }
}

impl EnigoInput {
    fn translate_key(&self, key: &str) -> Result<Key, InputError> {
        // Simple mapping for common keys
        Ok(match key.to_lowercase().as_str() {
            "ctrl" => Key::ControlLeft,
            "shift" => Key::ShiftLeft,
            "alt" => Key::AltLeft,
            "win" => Key::SuperLeft,
            "m" => Key::Key16, // just placeholder
            "up" => Key::ArrowUp,
            "down" => Key::ArrowDown,
            "left" => Key::ArrowLeft,
            "right" => Key::ArrowRight,
            "enter" => Key::Return,
            "esc" => Key::Escape,
            "tab" => Key::Tab,
            "space" => Key::Space,
            _ => {
                // For single characters
                let ch = key.chars().next().unwrap();
                Key::Char(ch)
            }
        })
    }

    fn char_to_key(&self, ch: char) -> Result<Key, InputError> {
        Ok(Key::Char(ch))
    }
}