use super::{ActionDefinition, ActionField, ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use serde_json::{Map, Value};

pub struct DelayPlugin;

#[async_trait]
impl ActionPlugin for DelayPlugin {
    fn definition(&self) -> ActionDefinition {
        ActionDefinition {
            kind: "delay".to_string(),
            name: "Delay".to_string(),
            description: "Wait before running the next action.".to_string(),
            fields: vec![ActionField::number("delay", "Seconds", 1.0, 0.0, 0.1)],
        }
    }

    fn validate_config(&self, config: &Map<String, Value>) -> Result<(), PluginError> {
        let seconds = config["delay"]
            .as_f64()
            .expect("field validation checked delay");
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(PluginError::InvalidConfiguration(
                "delay must be a finite, non-negative number of seconds".to_string(),
            ));
        }
        Ok(())
    }

    async fn execute(
        &self,
        config: &Map<String, Value>,
        _input: &dyn InputBackend,
    ) -> Result<(), PluginError> {
        let seconds = config["delay"]
            .as_f64()
            .expect("field validation checked delay");
        tokio::time::sleep(std::time::Duration::from_secs_f64(seconds)).await;
        Ok(())
    }
}
