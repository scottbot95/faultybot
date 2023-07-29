use crate::error::FaultyBotError;
use crate::settings::{SettingsContext, SettingsScopeKind, SettingsValue};
use crate::{Context, Error};
use poise::serenity_prelude::{ChannelId, UserId};

/// Manage settings for a specific scope
#[poise::command(slash_command, subcommands("get", "set"))]
pub async fn settings(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set settings for a specific scope
///
/// If `channel` and `user` are both unset, will manage guild-wide setting
///
/// Bot-wide per-user settings are currently not supported.
/// To change settings in your DMs, use per-channel settings
#[poise::command(slash_command)]
async fn set(
    ctx: Context<'_>,
    #[description = "Channel setting will be scoped to"]
    channel: Option<ChannelId>,
    #[description = "User setting will be scoped to"]
    user: Option<UserId>,
    setting: String,
    value: String,
) -> Result<(), Error> {
    todo!()
}

/// Get settings for a specific scope
///
/// If `channel` and `user` are both unset, will fetch current effective setting for caller
///
/// Bot-wide per-user settings are currently not supported.
/// To change settings in your DMs, use per-channel settings
#[poise::command(slash_command)]
async fn get(
    ctx: Context<'_>,
    channel: Option<ChannelId>,
    user: Option<UserId>,
    guild: Option<bool>,
    key: String,
) -> Result<(), Error> {
    let settings_provider = &ctx.data().settings_manager;
    let key = key.as_str();
    let setting = match (channel, user, guild.unwrap_or(false)) {
        (Some(channel_id), None, false) => {
            let value = settings_provider
                .get_channel::<String>(channel_id, key)
                .await?;
            SettingsValue::new(value, SettingsScopeKind::Channel(channel_id))
        }
        (None, Some(user_id), false) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Per-user settings not support outside a server. Please user per-channel settings for DMs".to_string();
                FaultyBotError::InvalidInputError(msg)
            })?;
            let value = settings_provider
                .get_member::<String>(guild_id, user_id, key)
                .await?;
            SettingsValue::new(value, SettingsScopeKind::Member(guild_id, user_id))
        }
        (None, None, true) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Cannot set guild-wide settings outside a guild".to_string();
                FaultyBotError::InvalidInputError(msg)
            })?;
            let value = settings_provider.get_guild(guild_id, key).await?;
            SettingsValue::new(value, SettingsScopeKind::Guild(guild_id))
        }
        (None, None, false) => {
            let ctx = SettingsContext {
                guild_id: ctx.guild_id(),
                channel_id: Some(ctx.channel_id()),
                user_id: Some(ctx.author().id),
            };
            settings_provider.get_value(ctx, key).await?
        }
        (_, _, _) => {
            let msg = "Please specify only one scope (channel, user, or guild)".to_string();
            return Err(Box::new(FaultyBotError::InvalidInputError(msg)));
        }
    };

    let value = setting
        .value()
        .clone()
        .unwrap_or_else(|| "None".to_string());

    let msg = format!(
        "Key: `{}`\nValue: `{}`\nReason: `{}`",
        key,
        value,
        setting.scope()
    );

    ctx.send(|b| b.content(msg).ephemeral(true)).await?;

    Ok(())
}