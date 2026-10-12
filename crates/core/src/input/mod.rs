use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("key error: {0}")]
    Key(String),
}

#[async_trait]
pub trait InputBackend: Send + Sync {
    async fn key_down(&self, key: &str) -> Result<(), InputError>;
    async fn key_up(&self, key: &str) -> Result<(), InputError>;
    async fn key_click(&self, key: &str) -> Result<(), InputError>;
    async fn type_text(&self, text: &str) -> Result<(), InputError>;
}

pub struct FakeInput {
    events: std::sync::Mutex<Vec<FakeEvent>>,
}

#[derive(Debug, Clone)]
pub struct FakeEvent {
    pub kind: FakeEventKind,
    pub key: String,
}

#[derive(Debug, Clone)]
pub enum FakeEventKind {
    KeyDown,
    KeyUp,
    Type,
}

impl FakeInput {
    pub fn new() -> Self {
        FakeInput { events: std::sync::Mutex::new(Vec::new()) }
    }

    pub fn events(&self) -> Vec<FakeEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[async_trait]
impl InputBackend for FakeInput {
    async fn key_down(&self, key: &str) -> Result<(), InputError> {
        self.events.lock().unwrap().push(FakeEvent { kind: FakeEventKind::KeyDown, key: key.to_string() });
        Ok(())
    }

    async fn key_up(&self, key: &str) -> Result<(), InputError> {
        self.events.lock().unwrap().push(FakeEvent { kind: FakeEventKind::KeyUp, key: key.to_string() });
        Ok(())
    }

    async fn key_click(&self, key: &str) -> Result<(), InputError> {
        self.key_down(key).await?;
        self.key_up(key).await?;
        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<(), InputError> {
        self.events.lock().unwrap().push(FakeEvent { kind: FakeEventKind::Type, key: text.to_string() });
        Ok(())
    }
}
