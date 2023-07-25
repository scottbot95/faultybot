use crate::database::Database;
use crate::settings::SettingsContext;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub struct GuildSettingsDatabaseSource {
    db: Database,
}

impl GuildSettingsDatabaseSource {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[poise::async_trait]
impl<'a> crate::util::AsyncSource<SettingsContext<'a>, serde_json::Value>
    for GuildSettingsDatabaseSource
{
    async fn get(
        &self,
        ctx: &SettingsContext<'a>,
    ) -> Result<Option<serde_json::Value>, crate::Error> {
        let guild_id: Option<i64> = ctx.guild_id.map(|v| unsafe { std::mem::transmute(v) });
        let entry = entities::guild_settings::Entity::find()
            .filter(entities::guild_settings::Column::GuildId.eq(guild_id))
            .filter(entities::guild_settings::Column::Key.eq(ctx.key))
            .one(self.db.connection())
            .await?;

        let value = entry.map(|m| m.value);

        Ok(value)
    }
}

pub struct ChannelSettingsDatabaseSource {
    db: Database,
}

impl ChannelSettingsDatabaseSource {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[poise::async_trait]
impl<'a> crate::util::AsyncSource<SettingsContext<'a>, serde_json::Value>
    for ChannelSettingsDatabaseSource
{
    async fn get(
        &self,
        ctx: &SettingsContext<'a>,
    ) -> Result<Option<serde_json::Value>, crate::Error> {
        let channel_id: i64 = unsafe { std::mem::transmute(ctx.channel_id.0) };
        let entry = entities::channel_settings::Entity::find()
            .filter(entities::channel_settings::Column::ChannelId.eq(channel_id))
            .filter(entities::channel_settings::Column::Key.eq(ctx.key))
            .one(self.db.connection())
            .await?;

        let value = entry.map(|m| m.value);

        Ok(value)
    }
}

pub struct GuildUserSettingsDatabaseSource {
    db: Database,
}

impl GuildUserSettingsDatabaseSource {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[poise::async_trait]
impl<'a> crate::util::AsyncSource<SettingsContext<'a>, serde_json::Value>
    for GuildUserSettingsDatabaseSource
{
    async fn get(
        &self,
        ctx: &SettingsContext<'a>,
    ) -> Result<Option<serde_json::Value>, crate::Error> {
        let guild_id: i64 = match ctx.guild_id {
            Some(guild_id) => unsafe { std::mem::transmute(guild_id.0) },
            None => return Ok(None),
        };
        let user_id: i64 = unsafe { std::mem::transmute(ctx.user_id.0) };
        let entry = entities::guild_user_settings::Entity::find()
            .filter(entities::guild_user_settings::Column::GuildId.eq(guild_id))
            .filter(entities::guild_user_settings::Column::UserId.eq(user_id))
            .filter(entities::guild_user_settings::Column::Key.eq(ctx.key))
            .one(self.db.connection())
            .await?;

        let value = entry.map(|m| m.value);

        Ok(value)
    }
}
