mod ping;

use serenity::async_trait;
use serenity::builder::{CreateApplicationCommand, CreateEmbed};
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::Context;
use std::str::FromStr;

pub enum SlashCommandResult {
    Simple(Option<String>),
    Embed(CreateEmbed),
}

#[async_trait]
pub trait SlashCommand {
    async fn run(ctx: &Context, command: &ApplicationCommandInteraction) -> SlashCommandResult;
    fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand;
}

pub enum SlashCommands {
    Ping,
}

impl SlashCommands {
    pub async fn run(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> SlashCommandResult {
        match self {
            SlashCommands::Ping => ping::Ping::run(ctx, command).await,
        }
    }

    pub fn register<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        match self {
            SlashCommands::Ping => ping::Ping::register(command),
        }
    }
}

impl FromStr for SlashCommands {
    type Err = ();

    fn from_str(command: &str) -> Result<Self, Self::Err> {
        match command {
            "ping" => Ok(Self::Ping),
            _ => Err(()),
        }
    }
}
