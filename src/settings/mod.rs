mod channel_settings;
pub(crate) mod config;
pub mod merge_strategies;
pub(crate) mod manager;

use poise::serenity_prelude::{ChannelId, GuildId, UserId};
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
        let msg = match self {
            SettingsScopeKind::Global => "Global",
            SettingsScopeKind::Guild(_) => "Guild",
            SettingsScopeKind::Channel(_) => "Channel",
            SettingsScopeKind::Member(_, _) => "Member",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug, Clone)]
pub struct SettingsValue<T> {
    kind: SettingsScopeKind,
    scope: Option<T>,
}

impl<T> SettingsValue<T> {
    pub fn new(scope: Option<T>, kind: SettingsScopeKind) -> Self {
        Self { kind, scope }
    }

    pub fn value(&self) -> &Option<T> {
        &self.scope
    }

    pub fn scope(&self) -> &SettingsScopeKind {
        &self.kind
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
    if lhs.scope.is_some() {
        lhs
    } else if let (Some(lhs_val), Some(rhs_val)) = (&lhs.scope, &rhs.scope) {
        match merge.merge(lhs_val, rhs_val) {
            MergeDecision::Left => lhs,
            MergeDecision::Right => rhs,
        }
    } else {
        rhs
    }
}
