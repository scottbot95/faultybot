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
        user_id: UserId,
        channel_id: ChannelId,
        guild_id: Option<GuildId>,
        action: &str,
    ) -> Result<(), Error> {
        let ctx = policy::PolicyContext {
            guild_id,
            channel_id,
            roles: vec![],
            user_id,
        };
        let policy = self.policy_manager
            .effective_policy(ctx, action)
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
