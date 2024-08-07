use crate::error::UserError;
use crate::permissions::policy::{Effect, Policy, PolicyContext, PolicyProvider, Principle};
use crate::permissions::{validate_access, Permission};
use crate::{Context, Error};
use poise::serenity_prelude::{ChannelId, RoleId, UserId};
use crate::util::say_ephemeral;

/// Manage permissions for a given principle
#[poise::command(slash_command, subcommands("get", "set"))]
pub async fn permissions(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set permissions for a given principle
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command)]
async fn set(
    ctx: Context<'_>,
    #[description = "Channel permission will be scoped to"] channel: Option<ChannelId>,
    #[description = "User permission will be scoped to"] user: Option<UserId>,
    #[description = "Role permission will be scoped to"] role: Option<RoleId>,
    #[description = "Permission to manage permissions for"] permission: PermissionChoice,
    #[description = "Extra specifier to limit permission (ie the name of a setting to grant manage for)"]
    specifier: Option<String>,
    #[description = "Effect to apply or `Unset` to revert to default permissions"]
    effect: EffectChoice,
    #[rename = "for"]
    #[description = "Amount of time from present to grant permission for (eg '2days 3hours'). Exclusive with `until`"]
    duration: Option<humantime::Duration>,
    #[description = "UTC Timestamp until the granted permissions expire (eg '2018-01-01 12:53:00'). Exclusive with `for`"]
    until: Option<humantime::Timestamp>,
) -> Result<(), Error> {
    let action = permission.into_permission(specifier).to_string();
    validate_access(&ctx, Permission::SetPermission(Some(action.clone()))).await?;

    let perm_manager = &ctx.data().permissions_manager;
    let policy_manager = perm_manager.as_ref();

    let principle = get_principle(&ctx, channel, user, role)?;

    let effect = if let Some(effect) = effect.into_effect() {
        effect
    } else {
        policy_manager
            .clear_policy(principle, action.clone())
            .await?;
        let msg = format!("Cleared policy for {} to do `{}`", principle, action);
        say_ephemeral(ctx, msg, true).await?;
        return Ok(());
    };

    let until = match (duration, until) {
        (Some(duration), None) => {
            let duration: std::time::Duration = duration.into();
            Some(std::time::SystemTime::now() + duration)
        }
        (None, Some(until)) => Some(until.into()),
        (None, None) => None,
        (_, _) => {
            let msg = "Cannot provide `for` and `until` simultaneously";
            return Err(UserError::invalid_input(msg).into());
        }
    } // Map is done in two stages since you must specify the timezone when converting from SystemTime
    .map(chrono::DateTime::<chrono::Utc>::from)
    .map(chrono::DateTime::<chrono::FixedOffset>::from);

    let policy = Policy {
        effect,
        principle,
        action,
        until,
    };
    policy_manager.save_policy(&policy).await?;

    let msg = format!("Saved policy {:?}", policy);
    say_ephemeral(ctx, msg, true).await?;

    Ok(())
}

/// Get permissions for a given principle
#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command)]
async fn get(
    ctx: Context<'_>,
    #[description = "Channel to fetch permissions for"] channel: Option<ChannelId>,
    #[description = "User to fetch permissions for"] user: Option<UserId>,
    #[description = "Role to fetch permissions for"] role: Option<RoleId>,
    #[description = "Permission to manage permissions for"] permission: PermissionChoice,
    #[description = "Extra specifier to limit permission (ie the name of a setting to grant manage for)"]
    specifier: Option<String>,
) -> Result<(), Error> {
    let action = permission.into_permission(specifier).to_string();
    validate_access(&ctx, Permission::GetPermission(Some(action.clone()))).await?;
    let policy_ctx = PolicyContext {
        guild_id: ctx.guild_id(),
        user_id: user,
        channel_id: channel,
        roles: role.map(|v| vec![v]).unwrap_or_default(),
    };

    let policy = ctx
        .data()
        .permissions_manager
        .as_ref()
        .effective_policy(ctx.serenity_context(), policy_ctx, action)
        .await?;

    let msg = format!("Effective policy: {:?}", policy);
    say_ephemeral(ctx, msg, true).await?;
    Ok(())
}

fn get_principle(
    ctx: &Context,
    channel: Option<ChannelId>,
    user: Option<UserId>,
    role: Option<RoleId>,
) -> Result<Principle, UserError> {
    match (channel, user, role) {
        (Some(channel_id), None, None) => Ok(Principle::Channel(channel_id)),
        (None, Some(user_id), None) => {
            if let Some(guild_id) = ctx.guild_id() {
                Ok(Principle::Member(guild_id, user_id))
            } else {
                let msg = "Please specify only one scope";
                Err(UserError::invalid_input(msg))
            }
        }
        (None, None, Some(role_id)) => Ok(Principle::Role(role_id)),
        // Guild-wide if in a guild or for the "channel" if in a DM
        (None, None, None) => {
            if let Some(guild_id) = ctx.guild_id() {
                Ok(Principle::Guild(guild_id))
            } else {
                Ok(Principle::Channel(ctx.channel_id()))
            }
        }
        _ => {
            let msg = "Please specify only one scope";
            Err(UserError::invalid_input(msg))
        }
    }
}

#[derive(poise::ChoiceParameter, derive_more::Display)]
pub enum EffectChoice {
    Allow,
    Deny,
    Unset,
}

impl EffectChoice {
    pub fn into_effect(self) -> Option<Effect> {
        match self {
            EffectChoice::Unset => None,
            EffectChoice::Allow => Some(Effect::Allow),
            EffectChoice::Deny => Some(Effect::Deny),
        }
    }
}

/// Enum use for choice selection when managing permissions.
/// Names are slightly different here to be more end-user friendly
#[derive(poise::ChoiceParameter, derive_more::Display)]
pub enum PermissionChoice {
    Chat,
    ManagePermissions,
    GetPermissions,
    ManageSettings,
    GetSettings,
    SendFeedback,
    CreatePersona,
    ListPersona,
    EditPersona,
    UsePersona,
    DeletePersona,
}

impl PermissionChoice {
    pub fn into_permission(self, specifier: Option<String>) -> Permission {
        match self {
            PermissionChoice::Chat => Permission::Chat(specifier),
            PermissionChoice::ManagePermissions => Permission::SetPermission(specifier),
            PermissionChoice::GetPermissions => Permission::GetPermission(specifier),
            PermissionChoice::ManageSettings => Permission::SetSetting(specifier),
            PermissionChoice::GetSettings => Permission::GetSetting(specifier),
            PermissionChoice::SendFeedback => Permission::SendFeedback(specifier),
            PermissionChoice::CreatePersona => Permission::CreatePersona,
            PermissionChoice::ListPersona => Permission::ListPersona,
            PermissionChoice::EditPersona => Permission::EditPersona(specifier),
            PermissionChoice::UsePersona => Permission::UsePersona(specifier),
            PermissionChoice::DeletePersona => Permission::DeletePersona(specifier),
        }
    }
}
