pub mod policy_manager;
pub mod policy;

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use crate::{Error};
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GuildId, UserId};
use crate::error::FaultyBotError;
use crate::permissions::policy::{Effect, PolicyProvider};
use crate::permissions::policy_manager::PolicyManager;

pub enum Permission {
    Chat,
    SetPermission(Option<String>),
    GetPermission(Option<String>),
    SetSetting(Option<String>),
    GetSetting(Option<String>),
}

impl Permission {
    pub fn action(&self) -> &str {
        match self {
            Permission::Chat => "chat",
            Permission::SetPermission(_) => "permissions.set",
            Permission::GetPermission(_) => "permissions.get",
            Permission::SetSetting(_) => "settings.set",
            Permission::GetSetting(_) => "settings.get",
        }
    }

    pub fn specifier(&self) -> Option<&str> {
        let specifier = match self {
            Permission::Chat => &None,
            Permission::SetPermission(specifier) => specifier,
            Permission::GetPermission(specifier) => specifier,
            Permission::SetSetting(specifier) => specifier,
            Permission::GetSetting(specifier) => specifier,
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

pub async fn validate_access(ctx: &crate::Context<'_>, permission: Permission) -> Result<(), Error> {
    ctx.data()
        .permissions_manager
        .enforce(ctx.author().id, ctx.channel_id(), ctx.guild_id(), permission)
        .await
}

pub struct PermissionsManager {
    policy_manager: PolicyManager,
    serenity_ctx: serenity::Context,
    owners: HashSet<UserId>,
}

impl PermissionsManager {
    pub fn new(db: crate::database::Database, ctx: serenity::Context, owners: HashSet<UserId>) -> Self {
        Self {
            policy_manager: PolicyManager::new(db),
            serenity_ctx: ctx,
            owners,
        }
    }

    pub async fn enforce(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        permission: Permission,
    ) -> Result<(), Error> {
        // Bot owners are super-user
        if self.owners.contains(&user_id) {
            return Ok(())
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
            guild_id.member(&self.serenity_ctx, user_id)
                .await?
                .roles
        } else {
            vec![]
        };

        let policy_ctx = policy::PolicyContext {
            guild_id,
            channel_id: Some(channel_id),
            roles,
            user_id: Some(user_id),
        };

        let policy = self.policy_manager
            .effective_policy(&self.serenity_ctx, policy_ctx, permission.to_string())
            .await?;

        match policy.effect {
            Effect::Allow => Ok(()),
            // Treat anything other than explicit allow as deny
            _ => Err(FaultyBotError::AccessDenied {
                reason: format!("{} does not have permissions for {}", policy.principle, permission)
            }.into())
        }
    }
}

impl AsRef<PolicyManager> for PermissionsManager {
    fn as_ref(&self) -> &PolicyManager {
        &self.policy_manager
    }
}
