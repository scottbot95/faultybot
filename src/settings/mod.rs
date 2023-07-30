mod channel_settings;
pub(crate) mod config;
pub mod merge_strategies;
pub(crate) mod manager;

use poise::serenity_prelude::{ChannelId, GuildId, Mentionable, UserId};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum SettingsScopeKind {
    Global,
    Guild(GuildId),
    Channel(ChannelId),
    Member(GuildId, UserId),
}

impl std::fmt::Display for SettingsScopeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsScopeKind::Global => write!(f, "Global"),
            SettingsScopeKind::Guild(_) => write!(f, "this server"),
            SettingsScopeKind::Channel(channel_id) => write!(f, "{}", channel_id.mention()),
            SettingsScopeKind::Member(_, user_id) => write!(f, "{} in this server", user_id.mention()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsValue<T> {
    scope: SettingsScopeKind,
    value: Option<T>,
}

impl<T> SettingsValue<T> {
    pub fn new(value: Option<T>, scope: SettingsScopeKind) -> Self {
        Self { value, scope }
    }

    pub fn value(&self) -> &Option<T> {
        &self.value
    }

    pub fn scope(&self) -> &SettingsScopeKind {
        &self.scope
    }
}

pub struct SettingsContext {
    pub guild_id: Option<GuildId>,
    pub channel_id: Option<ChannelId>,
    pub user_id: Option<UserId>,
}

pub enum MergeDecision {
    Left,
    Right,
}

pub trait MergeFn<V>: Send + Sync {
    fn merge(&self, lhs: &V, rhs: &V) -> MergeDecision;
}

impl<V, F> MergeFn<V> for F
where
    F: Fn(&V, &V) -> MergeDecision + Send + Sync,
{
    fn merge(&self, lhs: &V, rhs: &V) -> MergeDecision {
        self(lhs, rhs)
    }
}


fn merge_values<T: DeserializeOwned>(
    merge: &impl MergeFn<T>,
    lhs: SettingsValue<T>,
    rhs: SettingsValue<T>,
) -> SettingsValue<T> {
    match (&lhs.value, &rhs.value) {
        (None, Some(_)) => rhs,
        (Some(lhs_val), Some(rhs_val)) => match merge.merge(lhs_val, rhs_val) {
            MergeDecision::Left => lhs,
            MergeDecision::Right => rhs,
        }
        (_, _) => lhs,
    }
}
