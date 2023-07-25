use crate::gpt::Chat;
use metrics::{histogram, increment_counter};
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use crate::error::CooldownError;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;

pub(crate) struct Handler {
    // cooldowns: RwLock<Cooldowns<Data, Error>>,
}

impl Handler {
    pub fn new(config: poise::CooldownConfig) -> Self {
        Self {
            // cooldowns: RwLock::new(Cooldowns::new(config)),
        }
    }

    pub async fn handle_event<'a>(
        &self,
        ctx: &'a serenity::Context,
        event: &'a poise::Event<'a>,
        framework: poise::FrameworkContext<'a, Data, Error>,
        _data: &'a Data,
    ) -> Result<(), Error> {
        match event {
            poise::Event::Ready { .. } => {
                info!("Connected to discord!");
            }
            poise::Event::Resume { .. } => {
                info!("Connection resumed");
            }
            poise::Event::Message { new_message } => {
                let result = self.handle_message(ctx, framework, new_message).await;
                match result {
                    Ok(()) => {}
                    Err(err) => match err.downcast::<CooldownError>() {
                        Ok(cd_err) => {
                            warn!(
                                "Command on CD for user {}. {}",
                                new_message.author.name, cd_err
                            );
                        }
                        Err(err) => return Err(err),
                    },
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_message<'a>(
        &self,
        ctx: &'a serenity::Context,
        framework: poise::FrameworkContext<'a, Data, Error>,
        new_message: &'a serenity::Message,
    ) -> Result<(), Error> {
        // Ignore self messages
        if new_message.author.id == framework.bot_id {
            return Ok(());
        }

        // Only reply to DMs and direct mentions
        if new_message.guild_id.is_some() && !new_message.mentions_me(ctx).await? {
            return Ok(());
        }

        {
            // let time_remaining = self
            //     .cooldowns
            //     .read()
            //     .await
            //     .remaining_cooldown(CooldownContext {
            //         user_id: new_message.author.id,
            //         guild_id: new_message.guild_id,
            //         channel_id: new_message.channel_id,
            //         user_data: framework.user_data,
            //     })
            //     .await?;
            // if let Some(time_remaining) = time_remaining {
            //     return Err(CooldownError::new(time_remaining).into());
            // }
        }

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

        {
            // self.cooldowns
            //     .write()
            //     .await
            //     .start_cooldown(CooldownContext {
            //         user_id: new_message.author.id,
            //         guild_id: new_message.guild_id,
            //         channel_id: new_message.channel_id,
            //         user_data: framework.user_data,
            //     });
        }

        Ok(())
    }

    async fn reply_with_gpt_completion(
        ctx: &serenity::Context,
        message: &serenity::Message,
    ) -> Result<serenity::Message, Error> {
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

        if let Some(content) = chat_completion.content {
            let result = message
                .channel_id
                .send_message(&ctx, |builder| {
                    if message.guild_id.is_some() {
                        builder.reference_message(message);
                    }
                    builder.content(content)
                })
                .await?;
            return Ok(result);
        }

        panic!("FaultyBot does not support GPT function calls (yet?)")
    }
}
