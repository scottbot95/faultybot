mod permissions;
mod persona;
mod settings;
mod feedback;
mod permissions_v2;

use crate::{Context, Data, Error};
use crate::settings::config::FaultybotConfig;

pub fn commands_vec(config: &FaultybotConfig) -> Vec<poise::Command<Data, Error>> {
    let mut commands = vec![
        help(),
        permissions::permissions(),
        permissions_v2::permissions(),
        persona::persona(),
        settings::settings()
    ];

    if config.github.is_some() {
        commands.push(feedback::feedback());
    }

    commands
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
