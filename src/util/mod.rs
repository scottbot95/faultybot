use std::fmt::Display;
use poise::serenity_prelude::{GuildId, UserId};

/// Used to convert a value from an i64. Primarily used for serenity ID types
/// so we can serialize them for storing in Postgres which doesn't support unsigned types
pub trait Fromi64 {
    fn from_i64(int: i64) -> Self;
}

impl<T: From<u64>> Fromi64 for T {
    fn from_i64(int: i64) -> Self {
        let unsigned: u64 = unsafe { std::mem::transmute(int) };
        T::from(unsigned)
    }
}

/// Used to convert a value to an i64. Primarily used for serenity ID types
/// so we can serialize them for storing in Postgres which doesn't support unsigned types
pub trait Toi64 {
    fn to_i64(self) -> i64;
}

impl<T: Into<u64>> Toi64 for T {
    fn to_i64(self) -> i64 {
        unsafe { std::mem::transmute(T::into(self)) }
    }
}

pub trait OptionExt {
    fn to_string(self) -> String;
}

impl<T: Display> OptionExt for Option<T> {
    fn to_string(self) -> String {
        match self {
            None => "None".to_string(),
            Some(val) => val.to_string(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AuditInfo {
    pub user_id: UserId,
    pub guild_id: Option<GuildId>,
}

impl AuditInfo {
    pub fn as_metric_labels<'a>(&self) -> Vec<(&'a str, String)> {
        vec![
            ("user_id", self.user_id.to_string()),
            ("guild_id", self.guild_id.to_string()),
        ]
    }
}

impl From<&poise::serenity_prelude::Message> for AuditInfo {
    fn from(message: &poise::serenity_prelude::Message) -> Self {
        Self {
            user_id: message.author.id,
            guild_id: message.guild_id,
        }
    }
}

impl<'a, U, E> From<&poise::Context<'a, U, E>> for AuditInfo {
    fn from(ctx: &poise::Context<'a, U, E>) -> Self {
        Self {
            user_id: ctx.author().id,
            guild_id: ctx.guild_id(),
        }
    }

}