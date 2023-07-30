use std::sync::Arc;
use serde::de::DeserializeOwned;
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use sea_orm::{ColumnTrait, EntityTrait, IntoActiveValue, QueryFilter};
use sea_orm::sea_query::OnConflict;
use tracing::debug;
use entities::{channel_settings, guild_settings, member_settings};
use crate::{Error, settings};
use crate::settings::{merge_strategies, MergeFn, SettingsContext, SettingsScopeKind, SettingsValue};
use crate::util::Toi64;

pub struct SettingsManager {
    db: crate::database::Database,
    config: Arc<::config::Config>,
}

impl SettingsManager {
    pub fn new(config: Arc<::config::Config>, db: crate::database::Database) -> Self {
        Self { config, db }
    }

    pub async fn get_value<T: DeserializeOwned + std::fmt::Debug>(
        &self,
        ctx: SettingsContext,
        key: &str,
    ) -> Result<SettingsValue<T>, Error> {
        self.get_with_merge(ctx, key, merge_strategies::MostSpecific)
            .await
    }

    pub async fn get_with_merge<T: DeserializeOwned + std::fmt::Debug>(
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
            value = settings::merge_values(&merge, value, guild_val);
        }

        if let Some(channel_id) = ctx.channel_id {
            let channel_val = SettingsValue::new(
                self.get_channel(channel_id, key).await?,
                SettingsScopeKind::Channel(channel_id),
            );
            value = settings::merge_values(&merge, value, channel_val);
        }

        if let Some(guild_id) = ctx.guild_id {
            if let Some(user_id) = ctx.user_id {
                let member_val = SettingsValue::new(
                    self.get_member(guild_id, user_id, key).await?,
                    SettingsScopeKind::Member(guild_id, user_id),
                );
                value = settings::merge_values(&merge, value, member_val);
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
        let guild_id = guild_id.to_i64();
        let entry = guild_settings::Entity::find()
            .filter(guild_settings::Column::GuildId.eq(guild_id))
            .filter(guild_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }

    pub async fn set_guild<T: serde::Serialize>(&self, guild_id: GuildId, key: String, value: T) -> Result<(), Error> {
        let json = serde_json::to_value(value)?;
        let model = guild_settings::ActiveModel {
            guild_id: guild_id.to_i64().into_active_value(),
            key: key.into_active_value(),
            value: json.into_active_value(),
            ..Default::default()
        };

        guild_settings::Entity::insert(model)
            .on_conflict(OnConflict::columns(vec![
                guild_settings::Column::GuildId,
                guild_settings::Column::Key,
            ]).update_column(guild_settings::Column::Value)
                .to_owned())
            .exec(self.db.connection())
            .await?;

        Ok(())
    }

    pub async fn get_channel<T: DeserializeOwned>(
        &self,
        channel_id: ChannelId,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let channel_id = channel_id.to_i64();
        let entry = channel_settings::Entity::find()
            .filter(channel_settings::Column::ChannelId.eq(channel_id))
            .filter(channel_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }

    pub async fn set_channel<T: serde::Serialize>(&self, channel_id: ChannelId, key: String, value: T) -> Result<(), Error> {
        let json = serde_json::to_value(value)?;
        let model = channel_settings::ActiveModel {
            channel_id: channel_id.to_i64().into_active_value(),
            key: key.into_active_value(),
            value: json.into_active_value(),
            ..Default::default()
        };

        channel_settings::Entity::insert(model)
            .on_conflict(OnConflict::columns(vec![
                channel_settings::Column::ChannelId,
                channel_settings::Column::Key,
            ]).update_column(channel_settings::Column::Value)
                .to_owned())
            .exec(self.db.connection())
            .await?;

        Ok(())
    }

    pub async fn get_member<T: DeserializeOwned>(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let guild_id = guild_id.to_i64();
        let user_id = user_id.to_i64();

        let entry = member_settings::Entity::find()
            .filter(member_settings::Column::GuildId.eq(guild_id))
            .filter(member_settings::Column::UserId.eq(user_id))
            .filter(member_settings::Column::Key.eq(key))
            .one(self.db.connection())
            .await?;

        let value = match entry {
            Some(model) => Some(serde_json::from_value(model.value)?),
            None => None,
        };

        Ok(value)
    }

    pub async fn set_member<T: serde::Serialize>(&self, guild_id: GuildId, user_id: UserId, key: String, value: T) -> Result<(), Error> {
        let json = serde_json::to_value(value)?;
        let model = member_settings::ActiveModel {
            guild_id: guild_id.to_i64().into_active_value(),
            user_id: user_id.to_i64().into_active_value(),
            key: key.into_active_value(),
            value: json.into_active_value(),
            ..Default::default()
        };

        member_settings::Entity::insert(model)
            .on_conflict(OnConflict::columns(vec![
                member_settings::Column::GuildId,
                member_settings::Column::UserId,
                member_settings::Column::Key,
            ]).update_column(member_settings::Column::Value)
                .to_owned())
            .exec(self.db.connection())
            .await?;

        Ok(())
    }
}
