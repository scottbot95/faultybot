use crate::gpt::Chat;
use metrics::{histogram, increment_counter};
use tokio::time::Instant;
use tracing::{debug, error, info};

use poise::serenity_prelude as serenity;
use crate::{Data, Error};

pub(crate) struct Handler;

impl Handler {

    pub async fn handle_event<'a>(
        ctx: &'a serenity::Context,
        event: &'a poise::Event<'a>,
        framework: poise::FrameworkContext<'a, Data, Error>,
        _data: &'a Data
    ) ->  Result<(), Error> {
        match event {
            poise::Event::Ready { .. } => {
                info!("Connected to discord!");
            }
            poise::Event::Resume { .. } => {
                info!("Connection resumed");
            }
            poise::Event::Message { new_message } => {
                // Ignore self messages
                if new_message.author.id != framework.bot_id {
                    // Only reply to DMs and direct mentions
                    if new_message.guild_id.is_none() || new_message.mentions_me(ctx).await? {
                        let start = Instant::now();

                        let result = Self::reply_with_gpt_completion(ctx, new_message).await;

                        if let Err(err) = result {
                            increment_counter!("errors_total");
                            error!("Failed to send reply: {}", err);
                        } else {
                            increment_counter!("gpt_responses_total");
                        }

                        let duration = start.elapsed();
                        histogram!("gpt_response_seconds", duration.as_secs_f64());
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn reply_with_gpt_completion(ctx: &serenity::Context, message: &serenity::Message) -> Result<serenity::Message, Error> {
        increment_counter!("gpt_requests_total");

        let author = message
            .author_nick(ctx)
            .await
            .unwrap_or_else(|| message.author.name.clone());

        debug!("Received message from {}: `{}`", author, &message.content);

        let chat_completion = {
            let _typing = serenity::Typing::start(ctx.http.clone(), message.channel_id.0);

            let mut chat = Chat::from(ctx, "gpt-3.5-turbo", message).await;

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
