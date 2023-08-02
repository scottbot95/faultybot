use chrono::Utc;
use poise::serenity_prelude::{ChannelId, GuildId, RoleId, UserId};
use sea_orm::prelude::DateTime;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Principle {
    Global,
    Guild(GuildId),
    Channel(ChannelId),
    Role(RoleId),
    Member(GuildId, UserId),
}

pub enum Effect {
    Allow,
    Deny,
}

pub struct Policy {
    pub principle: Principle,
    pub action: String,
    pub effect: Effect,
    pub until: Option<chrono::DateTime<chrono::FixedOffset>>,
}

impl Policy {
    pub fn combined<I: Into<Self>>(policies: impl IntoIterator<Item=I>) -> Self {
        policies.into_iter()
            .map(I::into)
            .fold(Policy::default(), |acc, e|
                if let Some(until) = e.until {
                    let now = Utc::now();
                    if until > now {
                        e
                    } else {
                        acc
                    }
                } else if e.action.len() > acc.action.len() || e.principle > acc.principle {
                    e
                } else {
                    acc
                })
    }
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            principle: Principle::Global,
            action: "".to_string(),
            effect: Effect::Deny,
            until: None,
        }
    }
}


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PolicyContext {
    pub guild_id: Option<GuildId>,
    pub channel_id: ChannelId,
    pub roles: Vec<RoleId>,
    pub user_id: UserId,
}

#[poise::async_trait]
pub trait PolicyProvider<E> {
    async fn effective_policy(&self, ctx: PolicyContext, action: &str) -> Result<Policy, E> {
        // Start with default policy which denies all actions
        let mut policies = vec![Policy::default()];

        policies.extend(self.channel_policies(ctx.channel_id, action).await?);
        if let Some(guild_id) = ctx.guild_id {
            policies.extend(self.guild_policies(guild_id, action).await?);
            policies.extend(self.member_policies(guild_id, ctx.user_id, action).await?);
        }

        Ok(Policy::combined(policies))
    }

    async fn guild_policies(&self, guild_id: GuildId, action: &str) -> Result<Vec<Policy>, E>;
    async fn channel_policies(&self, channel_id: ChannelId, action: &str) -> Result<Vec<Policy>, E>;
    async fn role_policies(&self, role_id: RoleId, action: &str) -> Result<Vec<Policy>, E>;
    async fn member_policies(&self, guild_id: GuildId, user_id: UserId, action: &str) -> Result<Vec<Policy>, E>;
}
