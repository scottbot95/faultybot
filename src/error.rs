use derive_more::{Display, Error};
use std::time::Duration;

#[derive(Debug, Display, Clone, Error)]
#[display(fmt = "{:?} remaining on cooldown", remaining)]
pub struct CooldownError {
    remaining: Duration,
}

impl CooldownError {
    pub fn new(remaining: Duration) -> Self {
        Self { remaining }
    }

    #[allow(dead_code)]
    pub fn remaining(&self) -> Duration {
        self.remaining
    }
}

#[derive(Debug, Display)]
pub enum FaultyBotError {
    #[display(fmt = "Invalid input: {}", "0")]
    InvalidInput(String),
    #[display(fmt = "Access denied: {}", "reason")]
    AccessDenied {
        reason: String
    },
}

impl std::error::Error for FaultyBotError {}

impl FaultyBotError {
    pub fn invalid_input<T: Into<String>>(msg: T) -> Self {
        Self::InvalidInput(msg.into())
    }
    pub fn access_denied<T: Into<String>>(reason: T) -> Self {
        Self::AccessDenied { reason: reason.into() }
    }
}