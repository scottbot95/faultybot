pub mod policy_manager;
pub mod policy;

use crate::Error;
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use crate::error::FaultyBotError;
use crate::permissions::policy::{Effect, PolicyProvider};
use crate::permissions::policy_manager::PolicyManager;

pub struct PermissionsManager {
    policy_manager: PolicyManager,
}

impl PermissionsManager {
    pub fn new(db: crate::database::Database) -> Self {
        Self {
            policy_manager: PolicyManager::new(db)
        }
    }

    pub async fn enforce(
        &self,
        cache: impl AsRef<poise::serenity_prelude::Cache> + Send,
        user_id: UserId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        action: impl Into<String>,
    ) -> Result<(), Error> {
        let ctx = policy::PolicyContext {
            guild_id,
            channel_id: Some(channel_id),
            roles: vec![],
            user_id: Some(user_id),
        };

        let policy = self.policy_manager
            .effective_policy(cache, ctx, action.into())
            .await?;

        match policy.effect {
            Effect::Allow => Ok(()),
            // Treat anything other than explicit allow as deny
            _ => Err(FaultyBotError::AccessDenied {
                reason: "".to_string()
            }.into())
        }
    }
}

impl AsRef<PolicyManager> for PermissionsManager {
    fn as_ref(&self) -> &PolicyManager {
        &self.policy_manager
    }
}
