use std::fmt::{Display, Formatter};
use entities::sea_orm_active_enums::LlmModel;
use poise::serenity_prelude::{ChannelId, GuildId, Mentionable};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveEnum, IntoActiveValue, ActiveValue, ModelTrait, ActiveModelTrait, QueryOrder};
use sea_orm::sea_query::OnConflict;
use entities::{active_persona, persona};
use crate::Error;
use crate::error::{InternalError, UserError};
use crate::util::{Fromi64, Toi64};

pub struct PersonaManager {
    db: crate::Database,
}

impl PersonaManager {
    pub fn new(db: crate::Database) -> Self {
        Self { db }
    }

    /// Get the names of all the personas for a give [GuildId]
    pub async fn list_personas(&self, guild_id: GuildId) -> Result<Vec<Persona>, Error> {
        let names = persona::Entity::find()
            .filter(sea_orm::Condition::any()
                .add(persona::Column::GuildId.eq(guild_id.to_i64()))
                .add(persona::Column::GuildId.is_null()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Persona::from)
            .collect();

        Ok(names)
    }

    pub async fn get_by_name(&self, name: String, guild_id: GuildId) -> Result<Persona, Error> {
        let persona = self.find_model_by_name(&name, Some(guild_id))
            .await?
            .ok_or_else(|| UserError::not_found(format!("Persona `{}` does not exist", &name)))?;

        Ok(persona.into())
    }

    /// Load a specific persona and all the channels in which it is active
    pub async fn get_with_usage_by_name(&self, name: String, guild_id: GuildId) -> Result<PersonaMeta, Error> {
        let persona = self.find_model_by_name(&name, Some(guild_id))
            .await?
            .ok_or_else(|| UserError::not_found(format!("Persona `{}` does not exist", &name)))?;

        let active_settings = persona.find_related(active_persona::Entity)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|m| m.channel_id.map(ChannelId::from_i64))
            .collect::<Vec<_>>();

        let guild_active = active_settings
            .iter()
            .any(|c| c.is_none());

        let active_channels = active_settings
            .into_iter()
            .flatten()
            .collect();

        Ok(PersonaMeta {
            persona: persona.into(),
            guild_active,
            active_channels,
        })
    }

    pub async fn create(
        &self,
        name: String,
        description: Option<String>,
        guild_id: GuildId,
        prompt: String,
        model: LlmModel,
    ) -> Result<(), Error> {
        let existing = self.find_model_by_name(&name, Some(guild_id)).await?;
        if existing.is_some() {
            return Err(UserError::invalid_input(format!("Persona `{}` already exists", name)).into());
        }

        let persona = persona::ActiveModel {
            name: name.into_active_value(),
            description: description.into_active_value(),
            guild_id: Some(guild_id.to_i64()).into_active_value(),
            prompt: prompt.into_active_value(),
            model: ActiveValue::Set(model),
            builtin: false.into_active_value(),
            ..Default::default()
        };

        persona::Entity::insert(persona)
            .exec(self.db.connection())
            .await?;

        Ok(())
    }

    pub async fn update(&self, persona: Persona) -> Result<(), Error> {
        let model = persona::ActiveModel {
            id: persona.id.into_active_value(),
            name: persona.name().into_active_value(),
            prompt: persona.prompt.into_active_value(),
            model: ActiveValue::Set(persona.model),
            ..Default::default()
        };

        model.update(self.db.connection())
            .await?;

        Ok(())
    }

    pub async fn switch_active_person(&self, name: String, channel_id: Option<ChannelId>, guild_id: Option<GuildId>) -> Result<(), Error> {
        let persona = self.find_model_by_name(&name, guild_id)
            .await?
            .ok_or_else(|| UserError::not_found(format!("Persona `{}` does not exist", &name)))?;

        let guild_id = guild_id.map(GuildId::to_i64);
        let channel_id = channel_id.map(ChannelId::to_i64);

        let model = active_persona::ActiveModel {
            guild_id: guild_id.into_active_value(),
            channel_id: channel_id.into_active_value(),
            persona_id: persona.id.into_active_value(),
            ..Default::default()
        };

        active_persona::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([active_persona::Column::GuildId, active_persona::Column::ChannelId])
                    .update_columns([active_persona::Column::PersonaId])
                    .to_owned())
            .exec(self.db.connection())
            .await?;

        Ok(())
    }

    pub async fn get_active_persona(&self, channel_id: ChannelId, guild_id: Option<GuildId>) -> Result<Persona, Error> {
        let mut query = persona::Entity::find()
            .inner_join(active_persona::Entity)
            .order_by_asc(active_persona::Column::ChannelId)
            .order_by_asc(persona::Column::GuildId)
            .filter(sea_orm::Condition::any()
                .add(active_persona::Column::ChannelId.eq(channel_id.to_i64()))
                .add(active_persona::Column::ChannelId.is_null()));

        if let Some(guild_id) = guild_id {
            query = query.filter(
                sea_orm::Condition::any()
                    .add(active_persona::Column::GuildId.eq(guild_id.to_i64()))
                    .add(active_persona::Column::GuildId.is_null())
            );
        }

        let persona = query.one(self.db.connection())
            .await?
            .map(Persona::from)
            .ok_or_else(|| InternalError::unknown_persona(
                format!("Could not find persona for {:?} {:?}", channel_id, guild_id)
            ))?;

        Ok(persona)
    }

    async fn find_model_by_name(&self, name: &String, guild_id: Option<GuildId>) -> Result<Option<persona::Model>, Error> {
        let persona = persona::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(persona::Column::GuildId.eq(guild_id.map(GuildId::to_i64)))
                    .add(persona::Column::GuildId.is_null()))
            .filter(persona::Column::Name.eq(name))
            .one(self.db.connection())
            .await?;
        Ok(persona)
    }
}

pub struct PersonaMeta {
    pub persona: Persona,
    pub guild_active: bool,
    pub active_channels: Vec<ChannelId>,
}

impl Display for PersonaMeta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "Persona: {}\nModel: {}",
               self.persona.name, self.persona.model()
        )?;
        if let Some(desc) = &self.persona.description {
            write!(f, "\nDescription: {}", desc)?;
        }

        if self.persona.builtin {
            writeln!(f, "\nPrompt: `[REDACTED]`")?;
        } else {
            writeln!(f, "\nPrompt: {}", self.persona.prompt)?;
        }

        write!(f, "\nUsed in:")?;

        let mut used = false;
        if self.guild_active {
            write!(f, "\n- Server default")?;
            used = true;
        }

        if !self.active_channels.is_empty() {
            used = true;
            for channel in &self.active_channels {
                write!(f, "\n- {}", channel.mention())?;
            }
        }

        if !used {
            write!(f, "- Nowhere :(")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Persona {
    pub(crate) name: String,
    pub(crate) prompt: String,
    pub(crate) model: LlmModel,
    pub(crate) description: Option<String>,
    id: i32,
    builtin: bool,
    // pub(super) guild_id: serenity::GuildId,
}

impl Persona {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn model(&self) -> String {
        self.model.to_value()
    }

    pub fn description(&self) -> Option<String> { self.description.as_ref().cloned() }

    pub fn prompt(&self, bot_name: &str) -> String {
        self.prompt
            .replace("{bot_name}", bot_name)
            .replace("{persona}", self.prompt.as_str())
    }

    pub fn is_builtin(&self) -> bool {
        self.builtin
    }
}

impl From<persona::Model> for Persona {
    fn from(persona: persona::Model) -> Self {
        Self {
            name: persona.name,
            description: persona.description,
            prompt: persona.prompt,
            model: persona.model,
            id: persona.id,
            builtin: persona.builtin,
            // guild_id: serenity::GuildId::from_i64(model.guild_id)
        }
    }
}
