pub mod policy;
pub mod policy_manager;

use crate::error::UserError;
use crate::permissions::policy::{Effect, PolicyProvider};
use crate::permissions::policy_manager::PolicyManager;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GuildId, UserId};
use std::fmt::{Display, Formatter};
use poise::serenity_prelude::small_fixed_array::FixedArray;

pub enum Permission {
    Chat(Option<String>),
    SetPermission(Option<String>),
    GetPermission(Option<String>),
    SetSetting(Option<String>),
    GetSetting(Option<String>),
    SendFeedback(Option<String>),
    CreatePersona,
    ListPersona,
    EditPersona(Option<String>),
    UsePersona(Option<String>),
    DeletePersona(Option<String>),
    UseModel(Option<String>),
}

impl Permission {
    pub fn action(&self) -> &str {
        match self {
            Permission::Chat(_) => "chat",
            Permission::SetPermission(_) => "permissions.set",
            Permission::GetPermission(_) => "permissions.get",
            Permission::SetSetting(_) => "settings.set",
            Permission::GetSetting(_) => "settings.get",
            Permission::SendFeedback(_) => "feedback.send",
            Permission::CreatePersona => "persona.create",
            Permission::ListPersona => "persona.list",
            Permission::EditPersona(_) => "persona.edit",
            Permission::UsePersona(_) => "persona.use",
            Permission::DeletePersona(_) => "persona.delete",
            Permission::UseModel(_) => "model.use",
        }
    }

    pub fn specifier(&self) -> Option<&str> {
        let specifier = match self {
            Permission::Chat(specifier) => specifier,
            Permission::SetPermission(specifier) => specifier,
            Permission::GetPermission(specifier) => specifier,
            Permission::SetSetting(specifier) => specifier,
            Permission::GetSetting(specifier) => specifier,
            Permission::SendFeedback(specifier) => specifier,
            Permission::EditPersona(specifier) => specifier,
            Permission::UsePersona(specifier) => specifier,
            Permission::DeletePersona(specifier) => specifier,
            Permission::UseModel(specifier) => specifier,
            _ => &None,
        };

        specifier.as_ref().map(|s| s.as_str())
    }
}

impl Display for Permission {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let action = self.action();

        match self.specifier() {
            Some(specifier) => write!(f, "{}:{}", action, specifier),
            None => write!(f, "{}", action),
        }
    }
}

pub async fn validate_access(
    ctx: &crate::Context<'_>,
    permission: Permission,
) -> Result<(), Error> {
    ctx.data()
        .permissions_manager
        .enforce(
            ctx.framework(),
            ctx.author().id,
            ctx.channel_id(),
            ctx.guild_id(),
            permission,
        )
        .await
}

pub fn validate_owner(ctx: &crate::Context<'_>) -> Result<(), Error> {
    if ctx.framework().options.owners.contains(&ctx.author().id) {
        return Ok(());
    }

    Err(UserError::access_denied("Only the bot owner can do that").into())
}

pub struct PermissionsManager {
    policy_manager: PolicyManager,
}

impl PermissionsManager {
    pub fn new(
        db: crate::database::Database,
    ) -> Self {
        Self {
            policy_manager: PolicyManager::new(db),
        }
    }

    pub async fn enforce(
        &self,
        ctx: poise::FrameworkContext<'_, Data, Error>,
        user_id: UserId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        permission: Permission,
    ) -> Result<(), Error> {
        // Bot owners are super-user
        if ctx.options.owners.contains(&user_id) {
            return Ok(());
        }

        // FIXME Temp disable while testing
        /*
        // guild owners also count as super-users in their guild
        let guild_owner = guild_id.and_then(|id| self.serenity_ctx.cache.guild_field(id, |g| g.owner_id));
        if let Some(guild_owner) = guild_owner {
            if user_id == guild_owner {
                return Ok(());
            }
        }
         */

        let roles = if let Some(guild_id) = guild_id {
            guild_id.member(ctx.serenity_context, user_id).await?.roles
        } else {
            FixedArray::empty()
        }.into_vec();

        let policy_ctx = policy::PolicyContext {
            guild_id,
            channel_id: Some(channel_id),
            roles,
            user_id: Some(user_id),
        };

        let policy = self
            .policy_manager
            .effective_policy(ctx.serenity_context, policy_ctx, permission.to_string())
            .await?;

        match policy.effect {
            Effect::Allow => Ok(()),
            // Treat anything other than explicit allow as deny
            _ => Err(UserError::access_denied(format!(
                "{} does not have permissions for {}",
                policy.principle, permission
            ))
            .into()),
        }
    }
}

impl AsRef<PolicyManager> for PermissionsManager {
    fn as_ref(&self) -> &PolicyManager {
        &self.policy_manager
    }
}
