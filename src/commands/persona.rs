use std::fmt::{Display, Formatter};
use crate::{Context, Error};

use poise::Modal as _;
use std::fmt::Write as _;
use poise::serenity_prelude::{ChannelId, Mentionable};
use entities::sea_orm_active_enums::LlmModel;
use crate::permissions::{Permission, validate_access, validate_owner};
use crate::util::say_ephemeral;


#[derive(poise::Modal)]
#[name = "Edit Persona"]
pub struct PersonaModal {
    #[max_length=40]
    name: String,
    description: Option<String>,
    #[paragraph]
    prompt: String,
}

/// Manage personas within your serve
///
/// Note: Custom personas are currently only supported in a sever (no DMs)
#[poise::command(slash_command, subcommands("create", "edit", "list", "get", "switch"))]
pub async fn persona(_ctx: Context<'_>) -> Result<(), Error> { Ok(()) }

/// Create a new persona
#[poise::command(slash_command, guild_only)]
async fn create(
    ctx: Context<'_>,
    #[rename = "model"]
    #[description = "Which LLM Model to use for this persona (default GPT 3.5)"]
    model_choice: Option<ModelChoice>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // guild_only command

    validate_access(&ctx, Permission::CreatePersona).await?;
    validate_model_access(&ctx, &model_choice).await?;

    let ctx = match ctx {
        Context::Application(ctx) => ctx,
        _ => unreachable!()
    };

    let persona_manager = &ctx.data().persona_manager;
    let model_choice = model_choice.unwrap_or_default();

    let persona_data = PersonaModal::execute(ctx).await?;

    if persona_data.is_none() { return Ok(()); }
    let persona_data = persona_data.unwrap();

    persona_manager.create(
        persona_data.name.clone(),
        persona_data.description,
        guild_id,
        persona_data.prompt,
        model_choice.into(),
    ).await?;

    let msg = format!("Successfully created new persona: {}", persona_data.name);
    say_ephemeral(ctx.into(), msg, true).await?;

    Ok(())
}

/// Edit a persona
#[poise::command(slash_command, guild_only)]
async fn edit(
    ctx: Context<'_>,
    #[description = "Name of the persona to edit"]
    name: String,
    #[rename = "model"]
    #[description = "Change the LLM Model to use for this persona (default GPT 3.5)"]
    model_choice: Option<ModelChoice>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // guild_only command

    validate_access(&ctx, Permission::EditPersona(Some(name.clone()))).await?;
    validate_model_access(&ctx, &model_choice).await?;

    let persona_manager = &ctx.data().persona_manager;

    let existing_persona = persona_manager
        .get_by_name(name, guild_id)
        .await?;

    if existing_persona.is_builtin() {
        validate_owner(&ctx)?;
    }

    let modal_defaults = PersonaModal {
        name: existing_persona.name(),
        description: existing_persona.description(),
        prompt: existing_persona.prompt.clone()
    };

    let ctx = match ctx {
        Context::Application(ctx) => ctx,
        _ => unreachable!()
    };

    let persona_data = PersonaModal::execute_with_defaults(ctx, modal_defaults).await?;

    if persona_data.is_none() { return Ok(()); }
    let persona_data = persona_data.unwrap();

    let mut new_persona = existing_persona.clone();
    if let Some(model) = model_choice {
        new_persona.model = model.into();
    }
    new_persona.name = persona_data.name.clone();
    new_persona.prompt = persona_data.prompt;

    persona_manager.update(new_persona).await?;

    let msg = format!("Successfully updated persona: {}", persona_data.name);
    say_ephemeral(ctx.into(), msg, true).await?;

    Ok(())
}

/// Switch the active profile in a given channel or guild-wide
#[poise::command(slash_command, guild_only, rename="use")]
async fn switch(ctx: Context<'_>, name: String, channel: Option<ChannelId>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // guild_only command

    validate_access(&ctx, Permission::UsePersona(Some(name.clone()))).await?;

    ctx.data()
        .persona_manager
        .switch_active_person(name.clone(), channel, Some(guild_id))
        .await?;

    let msg = if let Some(channel_id) = channel {
        format!("I am now {} in {}", name, channel_id.mention())
    } else {
        format!("I am now {} server-wide", name)
    };
    ctx.say(msg).await?;

    Ok(())
}

/// List available personas
#[poise::command(slash_command, guild_only)]
async fn list(
    ctx: Context<'_>
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // guild_only command

    validate_access(&ctx, Permission::ListPersona).await?;

    let personas = ctx.data()
        .persona_manager
        .list_personas(guild_id)
        .await?;

    let mut msg = "Available personas:".to_string();
    for persona in personas {
        write!(&mut msg, "\n- {}", persona.name)?;
        if let Some(desc) = persona.description {
            write!(&mut msg, ": {}", desc)?;
        }
    }

    say_ephemeral(ctx, msg, true).await?;

    Ok(())
}

/// Fetch details about a given persona
#[poise::command(slash_command, guild_only)]
async fn get(ctx: Context<'_>, #[description = "Name of the persona to fetch details for"] name: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap(); // guild_only command

    validate_access(&ctx, Permission::ListPersona).await?;

    let meta = ctx.data()
        .persona_manager
        .get_with_usage_by_name(name, guild_id)
        .await?;

    say_ephemeral(ctx, meta.to_string(), true).await?;

    Ok(())
}

#[derive(PartialEq, poise::ChoiceParameter)]
pub enum ModelChoice {
    Gpt35,
    Gpt4
}

impl Display for ModelChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ModelChoice::Gpt35 => "GPT 3.5",
            ModelChoice::Gpt4 => "GPT 3"
        };
        write!(f, "{}", name)
    }
}

impl Default for ModelChoice {
    fn default() -> Self {
        Self::Gpt35
    }
}

impl From<ModelChoice> for LlmModel {
    fn from(choice: ModelChoice) -> Self {
        match choice {
            ModelChoice::Gpt35 => LlmModel::Gpt35Turbo,
            ModelChoice::Gpt4 => LlmModel::Gpt4,
        }
    }
}

async fn validate_model_access(ctx: &Context<'_>, model_choice: &Option<ModelChoice>) -> Result<(), Error> {
    if let Some(choice) = &model_choice {
        if choice != &ModelChoice::default() {
            validate_access(&ctx, Permission::UseModel(Some(choice.to_string()))).await?;
        }
    }
    Ok(())
}
