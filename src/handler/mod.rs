use std::error::Error;
use crate::commands::slash_commands::{SlashCommandResult, SlashCommands};
use crate::gpt::Chat;
use crate::BotInfoContainer;
use serenity::async_trait;
use serenity::http::Typing;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::prelude::command::Command;
use serenity::model::prelude::{Message, Ready, ResumedEvent};
use serenity::prelude::{Context, EventHandler};
use std::str::FromStr;
use metrics::{histogram, increment_counter};
use tokio::time::Instant;
use tracing::{debug, error, info};

pub(crate) struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, message: Message) {
        debug!("Received {:?}", message);

        if message.guild_id.is_some()
            && !message
                .mentions_me(&ctx)
                .await
                .expect("Failed to read mentions")
        {
            return;
        }

        {
            // Ignore message sent by the bot
            let data = ctx.data.read().await;
            if message.author.id == data.get::<BotInfoContainer>().unwrap().id {
                return;
            }
        }

        let author = message
            .author_nick(&ctx)
            .await
            .unwrap_or_else(|| message.author.name.clone());

        let start = Instant::now();

        let result = Self::reply_with_gpt_completion(&ctx, &message, author).await;

        if let Err(err) = result {
            increment_counter!("faultybot_errors_total");
            error!("Failed to send reply: {}", err);
        } else {
            increment_counter!("faultybot_gpt_responses_total");
        }

        let duration = start.elapsed();
        histogram!("faultybot_gpt_response_seconds", duration.as_secs_f64());
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);

        Command::set_global_application_commands(&ctx.http, |commands| {
            commands.create_application_command(|command| {
                SlashCommands::Ping.register(command).name("ping")
            });

            commands
        })
        .await
        .unwrap();
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        info!("Resumed");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // Indicate we're working on the command
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        .interaction_response_data(|message| message.content("Processing..."))
                })
                .await
                .unwrap();

            let command_result = match SlashCommands::from_str(&command.data.name) {
                Ok(slash_command) => slash_command.run(&ctx, &command).await,
                Err(_) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content("Unknown command")
                        })
                        .await
                        .unwrap();

                    return;
                }
            };

            let result = match command_result {
                SlashCommandResult::Simple(None) => {
                    command
                        .delete_original_interaction_response(&ctx.http)
                        .await
                        .unwrap();
                    return;
                }
                SlashCommandResult::Simple(Some(message)) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(message)
                        })
                        .await
                }
                SlashCommandResult::Embed(embed) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.set_embed(embed)
                        })
                        .await
                }
            };
            match result {
                Ok(_) => (),
                Err(e) => {
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(format!("Error: {e:?}"))
                        })
                        .await
                        .unwrap();
                }
            }
        }
    }
}

impl Handler {
    async fn reply_with_gpt_completion(ctx: &Context, message: &Message, author: String) -> Result<Message, Box<dyn Error>> {
        increment_counter!("faultybot_gpt_requests_total");
        debug!("Received message from {}: `{}`", author, &message.content);

        let chat_completion = {
            let _typing = Typing::start(ctx.http.clone(), message.channel_id.0);

            let mut chat = Chat::from(&ctx, "gpt-3.5-turbo", &message).await;

            chat.completion().await?
        };

        let result = message
            .channel_id
            .send_message(&ctx, |builder| {
                if message.guild_id.is_some() {
                    builder.reference_message(message);
                }
                builder.content(chat_completion.content)
            })
            .await?;
        Ok(result)
    }
}
