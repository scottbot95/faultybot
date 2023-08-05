use derive_more::{Display};
use std::time::Duration;

#[derive(Debug, Display)]
pub enum FaultyBotError {
    #[display(fmt = "Invalid input: {}", "0")]
    InvalidInput(String),
    #[display(fmt = "Access denied: {}", "reason")]
    AccessDenied {
        reason: String
    },
    #[display(fmt = "You're too fast. Please wait {:.1} seconds before retrying", "remaining.as_secs_f32()")]
    CooldownHit {
        remaining: Duration
    }
}

impl std::error::Error for FaultyBotError {}

impl FaultyBotError {
    pub fn invalid_input<T: Into<String>>(msg: T) -> Self {
        Self::InvalidInput(msg.into())
    }
    pub fn access_denied<T: Into<String>>(reason: T) -> Self {
        Self::AccessDenied { reason: reason.into() }
    }
    pub fn cooldown_hit(remaining: Duration) -> Self {
        Self::CooldownHit { remaining }
    }
}