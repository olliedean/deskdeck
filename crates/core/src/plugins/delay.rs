use super::{ActionPlugin, PluginError};
use crate::input::InputBackend;
use async_trait::async_trait;
use deskdeck_protocol::deck::Action;

pub struct DelayPlugin;

#[async_trait]
impl ActionPlugin for DelayPlugin {
    fn kind(&self) -> &'static str {
        "delay"
    }

    async fn execute(&self, action: &Action, _input: &dyn InputBackend) -> Result<(), PluginError> {
        let seconds = action.delay.ok_or_else(|| {
            PluginError::InvalidConfiguration("delay action requires a duration".to_string())
        })?;
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(PluginError::InvalidConfiguration(
                "delay must be a finite, non-negative number of seconds".to_string(),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_secs_f64(seconds)).await;
        Ok(())
    }
}
