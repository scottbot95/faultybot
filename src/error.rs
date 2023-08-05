use poise::serenity_prelude::UserId;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Invalid input: {message}")]
    InvalidInput { user: UserId, message: String },
    #[error("Access denied: {reason}")]
    AccessDenied { user: UserId, reason: String },
    #[error("You're too fast. Please wait {:.1} seconds before retrying", .remaining.as_secs_f32())]
    CooldownHit { user: UserId, remaining: Duration },
}

impl UserError {
    pub fn user(&self) -> &UserId {
        match self {
            UserError::InvalidInput { user, .. } => user,
            UserError::AccessDenied { user, .. } => user,
            UserError::CooldownHit { user, .. } => user,
        }
    }
}

impl UserError {
    pub fn invalid_input<T: Into<String>>(user: UserId, msg: T) -> Self {
        Self::InvalidInput {
            user,
            message: msg.into(),
        }
    }
    pub fn access_denied<T: Into<String>>(user: UserId, reason: T) -> Self {
        Self::AccessDenied {
            user,
            reason: reason.into(),
        }
    }
    pub fn cooldown_hit(user: UserId, remaining: Duration) -> Self {
        Self::CooldownHit { user, remaining }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub enum FaultyBotError {
    User(#[from] UserError),
    Serenity(#[from] poise::serenity_prelude::Error),
    Database(#[from] sea_orm::DbErr),
    Fmt(#[from] std::fmt::Error),
    OpenAI(#[from] openai::OpenAiError),
    Json(#[from] serde_json::Error),
    Config(#[from] config::ConfigError),
    Octocrab(#[from] octocrab::Error),
}
