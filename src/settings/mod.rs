mod channel_settings;
pub(crate) mod config;
pub mod merge_strategies;

use crate::Error;
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::de::DeserializeOwned;
use std::sync::Arc;

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

pub struct SettingsManager {
    db: crate::database::Database,
    config: Arc<::config::Config>,
}

impl SettingsManager {
    pub fn new(config: Arc<::config::Config>, db: crate::database::Database) -> Self {
        Self { config, db }
    }

    pub async fn get_value<T: DeserializeOwned>(
        &self,
        ctx: SettingsContext,
        key: &str,
    ) -> Result<SettingsValue<T>, Error> {
        self.get_with_merge(ctx, key, merge_strategies::MostSpecific)
            .await
    }

    pub async fn get_with_merge<T: DeserializeOwned>(
        &self,
        ctx: SettingsContext,
        key: &str,
        merge: impl MergeFn<T>,
    ) -> Result<SettingsValue<T>, Error> {
        let mut value = SettingsValue::new(self.get_global(key)?, SettingsScopeKind::Global);

        if let Some(guild_id) = ctx.guild_id {
            let guild_val = SettingsValue::new(
                self.get_guild(guild_id, key).await?,
                SettingsScopeKind::Guild(guild_id),
            );
            value = merge_values(&merge, value, guild_val);
        }

        if let Some(channel_id) = ctx.channel_id {
            let channel_val = SettingsValue::new(
                self.get_channel(channel_id, key).await?,
                SettingsScopeKind::Channel(channel_id),
            );
            value = merge_values(&merge, value, channel_val);
        }

        if let Some(guild_id) = ctx.guild_id {
            if let Some(user_id) = ctx.user_id {
                let member_val = SettingsValue::new(
                    self.get_member(guild_id, user_id, key).await?,
                    SettingsScopeKind::Member(guild_id, user_id),
                );
                value = merge_values(&merge, value, member_val);
            }
        }

        Ok(value)
    }

    pub fn get_global<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        let val = match self.config.get(format!("global.{}", key).as_str()) {
            Ok(val) => Some(val),
            Err(::config::ConfigError::NotFound(_)) => None,
            Err(err) => return Err(Box::new(err)),
        };

        Ok(val)
    }

    pub async fn get_guild<T: DeserializeOwned>(
        &self,
        guild_id: GuildId,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let guild_id = u64_to_i64(guild_id);
        let entry = entities::guild_settings::Entity::find()
            .filter(entities::guild_settings::Column::GuildId.eq(guild_id))
            .filter(entities::guild_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }

    pub async fn get_channel<T: DeserializeOwned>(
        &self,
        channel_id: ChannelId,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let channel_id = u64_to_i64(channel_id);
        let entry = entities::channel_settings::Entity::find()
            .filter(entities::channel_settings::Column::ChannelId.eq(channel_id))
            .filter(entities::channel_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }

    pub async fn get_member<T: DeserializeOwned>(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let guild_id = u64_to_i64(guild_id);
        let user_id = u64_to_i64(user_id);

        let entry = entities::member_settings::Entity::find()
            .filter(entities::member_settings::Column::GuildId.eq(guild_id))
            .filter(entities::member_settings::Column::UserId.eq(user_id))
            .filter(entities::member_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }
}

/// Transmutes a Into<u64> value into an i64. This preserves the binary
/// representation and simply re-interprets the memory. This is used as a hack
/// to get around the fact that Postgres doesn't actually support unsigned data
// TODO maybe we just switch to mysql?
fn u64_to_i64<T: Into<u64>>(num: T) -> i64 {
    unsafe { std::mem::transmute(num.into()) }
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
