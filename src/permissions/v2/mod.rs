use poise::serenity_prelude::{ChannelId, GuildId, RoleId, UserId};

mod policy;
mod resource;

pub use policy::*;
pub use resource::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Principle {
    guild_id: Option<GuildId>,
    channel_id: Option<ChannelId>,
    role_id: Option<RoleId>,
    user_id: Option<UserId>,
}
