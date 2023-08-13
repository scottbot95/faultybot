use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },
    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },
    #[error("You're too fast. Please wait {:.1} seconds before retrying", .remaining.as_secs_f32())]
    CooldownHit { remaining: Duration },
    #[error("Not Found: {message}")]
    NotFound { message: String },
}

impl UserError {
    pub fn invalid_input<T: Into<String>>(msg: T) -> Self {
        Self::InvalidInput {
            message: msg.into(),
        }
    }

    pub fn access_denied<T: Into<String>>(reason: T) -> Self {
        Self::AccessDenied {
            reason: reason.into(),
        }
    }

    pub fn cooldown_hit(remaining: Duration) -> Self {
        Self::CooldownHit { remaining }
    }

    pub fn not_found<T: ToString>(message: T) -> Self {
        let message = message.to_string();
        Self::NotFound { message }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("Unexpected interaction: {0}")]
    UnexpectedInteraction(String),
    #[error("Unknown persona: {0}")]
    UnknownPersona(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

impl InternalError {
    pub fn unknown_persona<T: Into<String>>(msg: T) -> Self {
        Self::UnknownPersona(msg.into())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub enum FaultyBotError {
    User(#[from] UserError),
    Internal(#[from] InternalError),
    Serenity(#[from] poise::serenity_prelude::Error),
    Database(#[from] sea_orm::DbErr),
    Fmt(#[from] std::fmt::Error),
    OpenAI(#[from] openai::OpenAiError),
    Json(#[from] serde_json::Error),
    Config(#[from] config::ConfigError),
    Octocrab(#[from] octocrab::Error),
    Boxed(Box<dyn std::error::Error + Send + Sync>),
}

impl FaultyBotError {
    pub fn boxed<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self::Boxed(Box::new(err))
    }
}