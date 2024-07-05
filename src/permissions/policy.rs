use chrono::Utc;
use poise::serenity_prelude::{ChannelId, GuildId, Mentionable, RoleId, UserId};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Principle {
    Global,
    Guild(GuildId),
    Channel(ChannelId),
    Role(RoleId),
    Member(GuildId, UserId),
}

impl Display for Principle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Principle::Global => write!(f, "Global"),
            Principle::Guild(_) => write!(f, "This guild"),
            Principle::Channel(channel_id) => write!(f, "{}", channel_id.mention()),
            Principle::Role(role_id) => write!(f, "{}", role_id.mention()),
            Principle::Member(_, user_id) => write!(f, "{}", user_id.mention()),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Effect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Policy {
    pub principle: Principle,
    pub action: String,
    pub effect: Effect,
    pub until: Option<chrono::DateTime<chrono::FixedOffset>>,
}

impl Policy {
    pub fn combined<I: Into<Self>>(
        policies: impl IntoIterator<Item = I>,
        cmp_role_id: impl Fn(RoleId, RoleId) -> Ordering,
    ) -> Self {
        policies
            .into_iter()
            .map(I::into)
            .reduce(|acc, e| acc.merge_with(e, &cmp_role_id))
            .unwrap_or_default()
    }

    /// Merge this [`Policy`] with another by choosing the most "specific" [`Policy`].
    /// Specificity is determined by, in-order:
    /// 1. Whichever [`Policy.action`] is longer
    /// 2. Whichever [`Policy.principle`] is greater according to [`PartialOrd`]
    /// 3. Whichever [`Principle::Role`] is greater Discord's role hierarchy
    /// 4. Otherwise, EXPLODE
    pub fn merge_with<I: Into<Self>>(
        self,
        other: I,
        cmp_role_id: impl Fn(RoleId, RoleId) -> std::cmp::Ordering,
    ) -> Self {
        let other = other.into();

        if !other.is_valid() {
            self
        } else if other.action.len() > self.action.len() || other.principle > self.principle {
            other
        } else if other.action.len() < self.action.len() || other.principle < self.principle {
            self
        } else if other.effect != self.effect {
            match (self.principle, other.principle) {
                (Principle::Role(self_role), Principle::Role(other_role)) => {
                    match cmp_role_id(self_role, other_role) {
                        Ordering::Less => other,
                        Ordering::Equal => {
                            tracing::warn!(
                            "Found matching policies for same role.\nPolicy1: {:?}\nPolicy2: {:?}",
                            self, other
                        );
                            // Just re-use exising role if they match (shouldn't be possible)}
                            self
                        }
                        Ordering::Greater => self,
                    }
                }
                _ => {
                    tracing::error!("Conflicting effects for policy with match specificity. Defaulting to first seen");
                    unreachable!();
                }
            }
        } else {
            tracing::error!(
                "Found conflicting policies.\nPolicy1: {:?}\nPolicy2: {:?}",
                self,
                other
            );
            unreachable!();
        }
    }

    pub fn is_valid(&self) -> bool {
        if let Some(until) = self.until {
            let now = Utc::now();
            until > now
        } else {
            true // no `until` field means it lasts forever
        }
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
    pub channel_id: Option<ChannelId>,
    pub roles: Vec<RoleId>,
    pub user_id: Option<UserId>,
}

#[poise::async_trait]
pub trait PolicyProvider<E> {
    /// Computes the effective policy for a specific interaction.
    /// Will default to [`Effect::Deny`].
    ///
    /// Guaranteed to return a policy
    async fn effective_policy(
        &self,
        serenity_context: &poise::serenity_prelude::Context,
        ctx: PolicyContext,
        action: String,
    ) -> Result<Policy, E>
    where
        E: Send + Sync + 'static,
    {
        // Start with default policy which denies all actions
        let mut policies = vec![Policy::default()];

        if let Some(channel_id) = ctx.channel_id {
            policies.extend(self.channel_policies(channel_id, action.clone()).await?);
        }

        if let Some(guild_id) = ctx.guild_id {
            policies.extend(self.guild_policies(guild_id, action.clone()).await?);

            if let Some(user_id) = ctx.user_id {
                policies.extend(
                    self.member_policies(guild_id, user_id, action.clone())
                        .await?,
                );
            }
        }

        let role_futs = ctx
            .roles
            .into_iter()
            // join_all awaits the futures in order
            // TODO could use tokio::spawn here to fetch policies across threads
            .map(|role_id| self.role_policies(role_id, action.clone()));

        let role_policies = futures::future::join_all(role_futs)
            .await
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        policies.extend(role_policies);

        let combined = Policy::combined(policies, |lhs, rhs| {
            let Some(guild) = ctx.guild_id.and_then(|g| serenity_context.cache.guild(g)) else {
                return Ordering::Equal;
            };
            let roles = &guild.roles;
            let lhs = roles.get(&lhs);
            let rhs = roles.get(&rhs);
            match (lhs, rhs) {
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (Some(lhs), Some(rhs)) => lhs.cmp(rhs),
                (None, None) => Ordering::Equal, // Just assume equal I guess w/e
            }
        });

        if !combined.is_valid() {
            tracing::error!(
                "Combining policies preferred an expired policy. {:?}",
                combined
            );
            return Ok(Policy::default());
        }

        Ok(combined)
    }

    async fn guild_policies(&self, guild_id: GuildId, action: String) -> Result<Vec<Policy>, E>;
    async fn channel_policies(
        &self,
        channel_id: ChannelId,
        action: String,
    ) -> Result<Vec<Policy>, E>;
    async fn role_policies(&self, role_id: RoleId, action: String) -> Result<Vec<Policy>, E>;
    async fn member_policies(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        action: String,
    ) -> Result<Vec<Policy>, E>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role_cmp(lhs: RoleId, rhs: RoleId) -> Ordering {
        lhs.cmp(&rhs)
    }

    #[test]
    fn policy_merge_by_principle() {
        let global = Policy {
            principle: Principle::Global,
            ..Default::default()
        };

        let guild = Policy {
            principle: Principle::Guild(5.into()),
            ..Default::default()
        };

        assert_eq!(global.merge_with(guild.clone(), role_cmp), guild);

        let channel = Policy {
            principle: Principle::Channel(123.into()),
            ..Default::default()
        };

        assert_eq!(guild.merge_with(channel.clone(), role_cmp), channel);
    }

    #[test]
    fn policy_merge_by_action_length() {
        let global_deny = Policy::default();

        let global_action_allow = Policy {
            action: "test.action".to_string(),
            ..Default::default()
        };

        assert_eq!(
            global_deny.merge_with(global_action_allow.clone(), role_cmp),
            global_action_allow
        );
    }
}
