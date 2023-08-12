use crate::error::UserError;
use crate::permissions::{validate_access, Permission};
use crate::settings::{SettingsContext, SettingsScopeKind, SettingsValue};
use crate::{Context, Error};
use poise::serenity_prelude::{ChannelId, UserId};
use crate::util::say_ephemeral;

/// Manage settings for a specific scope
///
/// Available settings:
/// - `chat.cooldown`: Cooldown between chat responses from FaultyBot
#[poise::command(slash_command, subcommands("get", "set", "unset"))]
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
    #[description = "Channel setting will be scoped to"] channel: Option<ChannelId>,
    #[description = "User setting will be scoped to"] user: Option<UserId>,
    key: String,
    #[description = "JSON encoded value. (text must be wrapped in quotes)"]
    value: serde_json::Value,
) -> Result<(), Error> {
    validate_access(&ctx, Permission::SetSetting(Some(key.clone()))).await?;

    let settings_manager = &ctx.data().settings_manager;
    let updated_scope = match (channel, user) {
        (Some(_), Some(_)) => {
            let msg = "Per-user-per-channel settings not supported. Please specify only one scope";
            return Err(UserError::invalid_input(msg).into());
        }
        (Some(channel_id), None) => {
            settings_manager
                .set_channel(channel_id, key.clone(), Some(value.clone()))
                .await?;
            SettingsScopeKind::Channel(channel_id)
        }
        (None, Some(user_id)) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Per-user settings not support outside a server. Please user per-channel settings for DMs";
                UserError::invalid_input(msg)
            })?;
            settings_manager
                .set_member(guild_id, user_id, key.clone(), Some(value.clone()))
                .await?;
            SettingsScopeKind::Member(guild_id, user_id)
        }
        (None, None) => {
            if let Some(guild_id) = ctx.guild_id() {
                settings_manager
                    .set_guild(guild_id, key.clone(), Some(value.clone()))
                    .await?;
                SettingsScopeKind::Guild(guild_id)
            } else {
                let channel_id = ctx.channel_id();
                settings_manager
                    .set_channel(channel_id, key.clone(), Some(value.clone()))
                    .await?;
                SettingsScopeKind::Channel(channel_id)
            }
        }
    };

    let msg = format!(
        "Successfully updated `{}` to `{}` for {}",
        key, value, updated_scope
    );
    say_ephemeral(ctx, msg, true).await?;

    Ok(())
}

/// Unset settings for a specific scope
///
/// If `channel` and `user` are both unset, will manage guild-wide setting
///
/// Bot-wide per-user settings are currently not supported.
/// To change settings in your DMs, use per-channel settings
#[poise::command(slash_command)]
async fn unset(
    ctx: Context<'_>,
    #[description = "Channel setting will be scoped to"] channel: Option<ChannelId>,
    #[description = "User setting will be scoped to"] user: Option<UserId>,
    key: String,
) -> Result<(), Error> {
    validate_access(&ctx, Permission::SetSetting(Some(key.clone()))).await?;
    let settings_manager = &ctx.data().settings_manager;

    let updated_scope = match (channel, user) {
        (Some(_), Some(_)) => {
            let msg = "Per-user-per-channel settings not supported. Please specify only one scope";
            return Err(UserError::invalid_input(msg).into());
        }
        (Some(channel_id), None) => {
            settings_manager
                .set_channel::<serde_json::Value>(channel_id, key.clone(), None)
                .await?;
            SettingsScopeKind::Channel(channel_id)
        }
        (None, Some(user_id)) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Per-user settings not support outside a server. Please user per-channel settings for DMs";
                UserError::invalid_input(msg)
            })?;
            settings_manager
                .set_member::<serde_json::Value>(guild_id, user_id, key.clone(), None)
                .await?;
            SettingsScopeKind::Member(guild_id, user_id)
        }
        (None, None) => {
            if let Some(guild_id) = ctx.guild_id() {
                settings_manager
                    .set_guild::<serde_json::Value>(guild_id, key.clone(), None)
                    .await?;
                SettingsScopeKind::Guild(guild_id)
            } else {
                let channel_id = ctx.channel_id();
                settings_manager
                    .set_channel::<serde_json::Value>(channel_id, key.clone(), None)
                    .await?;
                SettingsScopeKind::Channel(channel_id)
            }
        }
    };

    let msg = format!("Successfully unset `{}` for {}", key, updated_scope);
    say_ephemeral(ctx, msg, true).await?;

    Ok(())
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
    validate_access(&ctx, Permission::GetSetting(Some(key.clone()))).await?;
    let settings_manager = &ctx.data().settings_manager;
    let key = key.as_str();
    let setting: SettingsValue<serde_json::Value> = match (channel, user, guild.unwrap_or(false)) {
        (Some(channel_id), None, false) => {
            let value = settings_manager.get_channel(channel_id, key).await?;
            SettingsValue::new(value, SettingsScopeKind::Channel(channel_id))
        }
        (None, Some(user_id), false) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Per-user settings not support outside a server. Please user per-channel settings for DMs";
                UserError::invalid_input(msg)
            })?;
            let value = settings_manager.get_member(guild_id, user_id, key).await?;
            SettingsValue::new(value, SettingsScopeKind::Member(guild_id, user_id))
        }
        (None, None, true) => {
            let guild_id = ctx.guild_id().ok_or_else(|| {
                let msg = "Cannot set guild-wide settings outside a guild";
                UserError::invalid_input(msg)
            })?;
            let value = settings_manager.get_guild(guild_id, key).await?;
            SettingsValue::new(value, SettingsScopeKind::Guild(guild_id))
        }
        (None, None, false) => {
            let ctx = SettingsContext {
                guild_id: ctx.guild_id(),
                channel_id: Some(ctx.channel_id()),
                user_id: Some(ctx.author().id),
            };
            settings_manager.get_value(ctx, key).await?
        }
        (_, _, _) => {
            let msg = "Please specify only one scope (channel, user, or guild)";
            return Err(UserError::invalid_input(msg).into());
        }
    };

    let value = setting
        .value()
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "None".to_string());

    let msg = format!(
        "Key: `{}`\nValue: `{}`\nReason: {}",
        key,
        value,
        setting.scope()
    );

    say_ephemeral(ctx, msg, true).await?;

    Ok(())
}
