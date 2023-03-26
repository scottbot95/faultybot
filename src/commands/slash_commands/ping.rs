use serenity::async_trait;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use crate::commands::slash_commands::{SlashCommand, SlashCommandResult};

pub(crate) struct Ping;

#[async_trait]
impl SlashCommand for Ping {
    async fn run(_ctx: &Context, _command: &ApplicationCommandInteraction) -> SlashCommandResult {
        SlashCommandResult::Simple(Some("I'm not dead yet!".to_string()))
    }

    fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
        command.description("Check if the bot is alive")
    }
}