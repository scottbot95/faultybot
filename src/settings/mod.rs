mod channel_settings;
pub(crate) mod config;
mod database_source;

use crate::settings::database_source::{
    ChannelSettingsDatabaseSource, GuildSettingsDatabaseSource, GuildUserSettingsDatabaseSource,
};
use crate::util::{CachingMap, CascadingMap};
use crate::Error;
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct ChatSettings {
    pub cooldown_sec: f32,
}

#[derive(Debug, Default, Deserialize)]
pub struct GlobalSettings {}

#[derive(Clone, Default, Debug, Hash, Eq, PartialEq)]
pub struct SettingsContext<'a> {
    pub guild_id: Option<GuildId>,
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub key: &'a str,
}

pub struct SettingsProvider {
    cache: CachingMap<SettingsContext<'static>, serde_json::Value>,
}

impl SettingsProvider {
    pub fn new(
        cache_capacity: u64,
        config: ::config::Config,
        db: crate::database::Database,
    ) -> Self {
        let data = CascadingMap::new(vec![
            Box::pin(ConfigSettingsSource {
                config,
                prefix: Some("global_settings.".to_owned()),
            }),
            Box::pin(GuildSettingsDatabaseSource::new(db.clone())),
            Box::pin(ChannelSettingsDatabaseSource::new(db.clone())),
            Box::pin(GuildUserSettingsDatabaseSource::new(db)),
        ]);
        Self {
            cache: CachingMap::new(cache_capacity, data),
        }
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        ctx: SettingsContext<'static>,
    ) -> Result<Option<T>, crate::Error> {
        let json = self.cache.get(ctx).await?;

        let parsed = match json {
            Some(json) => Some(serde_json::from_value(json)?),
            None => None,
        };

        Ok(parsed)
    }

    pub async fn get_with<T: DeserializeOwned>(
        &self,
        ctx: SettingsContext<'static>,
        default: impl FnOnce() -> T,
    ) -> Result<T, crate::Error> {
        let value = self.get(ctx).await?.unwrap_or_else(default);

        Ok(value)
    }

    /// Invalidate the cache for a specific guild. Call after changing a guild-level setting
    pub fn invalidate_guild(&self, guild_id: GuildId) {
        self.cache
            .invalidate_entries_if(move |k, _v| match k.guild_id {
                Some(key_guild) => key_guild == guild_id,
                None => false,
            });
    }

    /// Invalidate the cache for a specific channel. Call after changing a channel-level setting
    pub fn invalidate_channel(&self, channel_id: ChannelId) {
        self.cache
            .invalidate_entries_if(move |k, _v| k.channel_id == channel_id);
    }

    /// Invalidate the cache for a specific guild+user combo. Call after changing a user-level setting
    pub fn invalidate_guild_user(&self, guild_id: GuildId, user_id: UserId) {
        self.cache
            .invalidate_entries_if(move |k, _v| match k.guild_id {
                Some(key_guild) => key_guild == guild_id && k.user_id == user_id,
                None => false,
            });
    }
}

struct ConfigSettingsSource {
    config: ::config::Config,
    prefix: Option<String>,
}

#[poise::async_trait]
impl crate::util::AsyncSource<SettingsContext<'static>, serde_json::Value>
    for ConfigSettingsSource
{
    async fn get(
        &self,
        key: &SettingsContext<'static>,
    ) -> Result<Option<serde_json::Value>, Error> {
        let key = match &self.prefix {
            Some(prefix) => format!("{}{}", prefix, key.key),
            None => key.key.to_owned(),
        };

        match self.config.get(key.as_str()) {
            Ok(val) => Ok(Some(val)),
            Err(::config::ConfigError::NotFound(_)) => Ok(None),
            Err(err) => Err(Box::new(err)),
        }
    }
}
