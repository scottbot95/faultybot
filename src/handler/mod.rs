use crate::gpt::Chat;
use metrics::{histogram, increment_counter};
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::error::CooldownError;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;

const COOLDOWN_KEY: &str = "chat.cooldown";

pub(crate) struct Handler {
    cooldowns: RwLock<poise::Cooldowns>,
}

impl Handler {
    pub fn new(default_config: poise::CooldownConfig) -> Self {
        Self {
            cooldowns: RwLock::new(poise::Cooldowns::new(default_config)),
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
                let result = self
                    .handle_message(ctx.clone(), framework.clone(), new_message.clone())
                    .await;
                match result {
                    Ok(()) => {}
                    Err(err) => match err.downcast::<CooldownError>() {
                        Ok(cd_err) => {
                            let msg = format!(
                                "You're too fast. Please wait {:.1} seconds before retrying",
                                cd_err.remaining().as_secs_f32()
                            );
                            new_message.reply(ctx, msg).await?;
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
        ctx: serenity::Context,
        framework: poise::FrameworkContext<'a, Data, Error>,
        new_message: serenity::Message,
    ) -> Result<(), Error> {
        // Ignore self messages
        if new_message.author.id == framework.bot_id {
            return Ok(());
        }

        // Only reply to DMs and direct mentions
        if new_message.guild_id.is_some() && !new_message.mentions_me(ctx.clone()).await? {
            return Ok(());
        }

        let cd_ctx = poise::CooldownContext {
            user_id: new_message.author.id,
            guild_id: new_message.guild_id,
            channel_id: new_message.channel_id,
        };

        {
            let new_config = self.get_config(cd_ctx.clone(), framework.user_data).await?;
            self.cooldowns.write().await.set_config(new_config);
        }

        {
            let time_remaining = self
                .cooldowns
                .read()
                .await
                .remaining_cooldown(cd_ctx.clone());
            if let Some(time_remaining) = time_remaining {
                return Err(CooldownError::new(time_remaining).into());
            }
        }

        let start = Instant::now();

        let result = Self::reply_with_gpt_completion(&ctx, &new_message).await;

        if let Err(err) = result {
            increment_counter!("errors_total");
            error!("Failed to send reply: {}", err);
        } else {
            increment_counter!("gpt_responses_total");
        }

        let duration = start.elapsed();
        histogram!("gpt_response_seconds", duration.as_secs_f64());

        {
            self.cooldowns.write().await.start_cooldown(cd_ctx);
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
                        builder
                            .reference_message(message)
                            // Disallow mentions
                            .allowed_mentions(|f| f);
                    }
                    builder.content(content)
                })
                .await?;
            return Ok(result);
        }

        panic!("FaultyBot does not support GPT function calls (yet?)")
    }

    async fn get_config(
        &self,
        ctx: poise::CooldownContext,
        user_data: &Data,
    ) -> Result<poise::CooldownConfig, Error> {
        let config = poise::CooldownConfig {
            global: user_data
                .settings_manager
                .get_global(COOLDOWN_KEY)?
                .map(Duration::from_secs_f32),
            // don't support bot-wide per-user settings. You can have settings unique to DMs by channel though
            user: None,
            guild: match ctx.guild_id {
                Some(guild_id) => user_data
                    .settings_manager
                    .get_guild(guild_id, COOLDOWN_KEY)
                    .await?
                    .map(Duration::from_secs_f32),
                None => None,
            },
            channel: user_data
                .settings_manager
                .get_channel(ctx.channel_id, COOLDOWN_KEY)
                .await?
                .map(Duration::from_secs_f32),
            member: match ctx.guild_id {
                Some(guild_id) => user_data
                    .settings_manager
                    .get_member(guild_id, ctx.user_id, COOLDOWN_KEY)
                    .await?
                    .map(Duration::from_secs_f32),
                None => None,
            },
            __non_exhaustive: (),
        };

        Ok(config)
    }
}
