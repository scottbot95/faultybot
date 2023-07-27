//! Infrastructure for command cooldowns
//! Copied and modified from https://github.com/serenity-rs/poise/blob/a033d140a8d9cbfab465db3272f5fd01876518c2/src/cooldown.rs

use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Subset of [`crate::Context`] so that [`Cooldowns`] can be used without requiring a full [Context](`crate::Context`)
/// (ie from within an `event_handler`)
#[derive(derivative::Derivative)]
#[derivative(Clone(bound = ""))]
#[derive(PartialEq, Eq, Debug, Hash)]
pub struct CooldownContext {
    /// The user associated with this request
    pub user_id: serenity::UserId,
    /// The guild this request originated from or `None`
    pub guild_id: Option<serenity::GuildId>,
    /// The channel associated with this request
    pub channel_id: serenity::ChannelId,
}

/// Configuration struct for [`Cooldowns`]
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CooldownConfig {
    /// This cooldown operates on a global basis
    pub global: Option<Duration>,
    /// This cooldown operates on a per-user basis
    pub user: Option<Duration>,
    /// This cooldown operates on a per-guild basis
    pub guild: Option<Duration>,
    /// This cooldown operates on a per-channel basis
    pub channel: Option<Duration>,
    /// This cooldown operates on a per-member basis
    pub member: Option<Duration>
}

#[poise::async_trait]
pub trait CooldownConfigProvider<U, E> {
    async fn get_config(&self, ctx: CooldownContext, user_data: &U) -> Result<CooldownConfig, E>;
}

#[poise::async_trait]
impl<U, E, F, Fut> CooldownConfigProvider<U, E> for F
where
    F: Fn(CooldownContext, &U) -> Fut + Send + Sync,
    Fut: Future<Output=Result<CooldownConfig, E>> + Send,
    U: Sync {

    async fn get_config(&self, ctx: CooldownContext, user_data: &U) -> Result<CooldownConfig, E> {
        self(ctx, user_data).await
    }
}

#[poise::async_trait]
impl<U, E> CooldownConfigProvider<U, E> for CooldownConfig {
    async fn get_config(&self, _ctx: CooldownContext, _user_data: &U) -> Result<CooldownConfig, E> {
        Ok(self.clone())
    }
}

/// Handles cooldowns for a single command
///
/// You probably don't need to use this directly. `#[poise::command]` automatically generates a
/// cooldown handler.
#[derive(derivative::Derivative)]
#[derivative(Debug(bound = ""))]
#[derivative(Clone(bound = ""))]
pub struct Cooldowns<U, E> {
    /// Used to lookup the cooldown durations based off the [`CooldownContext`]
    #[derivative(Debug = "ignore")]
    cooldown_provider: Arc<dyn CooldownConfigProvider<U, E> + Send + Sync>,

    /// Stores the timestamp of the last global invocation
    global_invocation: Option<Instant>,
    /// Stores the timestamps of the last invocation per user
    user_invocations: HashMap<serenity::UserId, Instant>,
    /// Stores the timestamps of the last invocation per guild
    guild_invocations: HashMap<serenity::GuildId, Instant>,
    /// Stores the timestamps of the last invocation per channel
    channel_invocations: HashMap<serenity::ChannelId, Instant>,
    /// Stores the timestamps of the last invocation per member (user and guild)
    member_invocations: HashMap<(serenity::UserId, serenity::GuildId), Instant>,
}

impl<U: Sync + 'static, E> Default for Cooldowns<U, E> {
    fn default() -> Self {
        Self {
            cooldown_provider: Arc::new(CooldownConfig::default()),
            global_invocation: Default::default(),
            user_invocations: Default::default(),
            guild_invocations: Default::default(),
            channel_invocations: Default::default(),
            member_invocations: Default::default(),
        }
    }
}
impl<U, E> Cooldowns<U, E> {
    /// Create a new cooldown handler with the given cooldown durations
    pub fn new(config_provider: impl CooldownConfigProvider<U, E> + Send + Sync + 'static) -> Self {
        Self {
            cooldown_provider: Arc::new(config_provider),

            global_invocation: None,
            user_invocations: HashMap::new(),
            guild_invocations: HashMap::new(),
            channel_invocations: HashMap::new(),
            member_invocations: HashMap::new(),
        }
    }
}

impl<U, E> Cooldowns<U, E> {
    /// Queries the cooldown buckets and checks if all cooldowns have expired and command
    /// execution may proceed. If not, Some is returned with the remaining cooldown.
    /// Forwards [`Err`]'s from [`CooldownConfigProvider`], otherwise always returns [`Ok`]
    pub async fn remaining_cooldown(
        &self,
        ctx: CooldownContext,
        user_data: &U,
    ) -> Result<Option<Duration>, E> {
        let cooldowns = self.cooldown_provider.get_config(ctx.clone(), user_data).await?;
        let mut cooldown_data = vec![
            (cooldowns.global, self.global_invocation),
            (
                cooldowns.user,
                self.user_invocations.get(&ctx.user_id).copied(),
            ),
            (
                cooldowns.channel,
                self.channel_invocations.get(&ctx.channel_id).copied(),
            ),
        ];

        if let Some(guild_id) = ctx.guild_id {
            cooldown_data.push((
                cooldowns.guild,
                self.guild_invocations.get(&guild_id).copied(),
            ));
            cooldown_data.push((
                cooldowns.member,
                self.member_invocations
                    .get(&(ctx.user_id, guild_id))
                    .copied(),
            ));
        }

        let remaining = cooldown_data
            .iter()
            .filter_map(|&(cooldown, last_invocation)| {
                let duration_since = Instant::now().saturating_duration_since(last_invocation?);
                let cooldown_left = cooldown?.checked_sub(duration_since)?;
                Some(cooldown_left)
            })
            .max();

        Ok(remaining)
    }

    /// Indicates that a command has been executed and all associated cooldowns should start running
    pub fn start_cooldown(&mut self, ctx: CooldownContext) {
        let now = Instant::now();

        self.global_invocation = Some(now);
        self.user_invocations.insert(ctx.user_id, now);
        self.channel_invocations.insert(ctx.channel_id, now);

        if let Some(guild_id) = ctx.guild_id {
            self.guild_invocations.insert(guild_id, now);
            self.member_invocations.insert((ctx.user_id, guild_id), now);
        }
    }
}

impl From<serenity::Message> for CooldownContext {
    fn from(message: serenity::Message) -> Self {
        CooldownContext {
            user_id: message.author.id,
            guild_id: message.guild_id,
            channel_id: message.channel_id,
        }
    }
}
