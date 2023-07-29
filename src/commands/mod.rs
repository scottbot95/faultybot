mod permissions;
mod settings;

use crate::{Context, Data, Error};

pub fn commands_vec() -> Vec<poise::Command<Data, Error>> {
    vec![help(), permissions::permissions(), settings::settings()]
}

/// Show this help menu
#[poise::command(slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is a ChatGPT-powered Discord chat bot made by FaultyMuse.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}
