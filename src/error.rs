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
