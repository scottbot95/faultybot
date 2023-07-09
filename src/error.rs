use std::time::Duration;
use derive_more::{Display, Error};

#[derive(Debug, Display, Clone, Error)]
#[display(fmt = "{:?} remaining on cooldown", remaining)]
pub struct CooldownError {
    remaining: Duration,
}

impl CooldownError {
    pub fn new(remaining: Duration) -> Self {
        Self { remaining }
    }

    pub fn remaining(&self) -> Duration {
        self.remaining
    }
}