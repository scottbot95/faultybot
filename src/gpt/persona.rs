use entities::sea_orm_active_enums::LlmModel;
use std::collections::HashMap;
use poise::serenity_prelude::{ChannelId, GuildId};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveEnum};
use entities::{active_persona, persona};
use crate::Error;
use crate::util::Toi64;

pub struct PersonaManager {
    db: crate::Database,
}

impl PersonaManager {
    pub fn new(db: crate::Database) -> Self {
        Self { db }
    }

    pub async fn load(&self, channel_id: ChannelId, guild_id: Option<GuildId>) -> Result<Persona, Error> {
        let persona = self.get_active_persona(channel_id, guild_id).await?;

        Ok(persona)
    }

    async fn get_active_persona(&self, channel_id: ChannelId, guild_id: Option<GuildId>) -> Result<Persona, Error> {
        let mut query = persona::Entity::find()
            .left_join(active_persona::Entity)
            .filter(active_persona::Column::ChannelId.eq(channel_id.to_i64()));

        if let Some(guild_id) = guild_id {
            query = query
                .filter(active_persona::Column::GuildId.eq(guild_id.to_i64()))
                .filter(persona::Column::GuildId.eq(guild_id.to_i64()));
        }

        let persona = query.one(self.db.connection())
            .await?
            .map(Persona::from)
            .unwrap_or_default();

        Ok(persona)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Persona {
    name: String,
    prompt: String,
    model: LlmModel,
    // pub(super) guild_id: serenity::GuildId,
}

impl Persona {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn model(&self) -> String {
        self.model.to_value()
    }

    pub fn prompt(&self, bot_name: &str) -> String {
        self.prompt
            .replace("{bot_name}", bot_name)
            .replace("{persona}", self.prompt.as_str())
    }
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            name: "Sassy".to_string(),
            prompt: "You are {bot_name}, a helpful assistant build into a Discord bot.
                You are helpful, but your responses are always sassy and sometimes rude."
                .trim()
                .to_string(),
            model: LlmModel::Gpt35Turbo,
        }
    }
}

impl From<persona::Model> for Persona {
    fn from(persona: persona::Model) -> Self {
        Self {
            name: persona.name,
            prompt: persona.prompt,
            model: persona.model,
            // guild_id: serenity::GuildId::from_i64(model.guild_id)
        }
    }
}
