use crate::gpt::Chat;
use metrics::{histogram, increment_counter};
use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::error::{FaultyBotError, UserError};
use crate::permissions::Permission;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;
use crate::util::AuditInfo;

const COOLDOWN_KEY: &str = "chat.cooldown";

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) -> Result<(), Error> {
    match error {
        poise::FrameworkError::Command { ctx, error, .. } => {
            handle_error(error, (&ctx).into(), |msg| async move {
                ctx.send(|b| b.content(msg).ephemeral(true)).await?;
                Ok(())
            })
            .await?;
        }
        error => {
            increment_counter!("errors_total");
            poise::builtins::on_error(error).await?
        }
    };

    Ok(())
}

async fn handle_error<F, Fut>(error: FaultyBotError, audit_info: AuditInfo, send_message: F) -> Result<(), serenity::Error>
where
    Fut: Future<Output = Result<(), serenity::Error>>,
    F: FnOnce(String) -> Fut,
{
    let metric_labels = audit_info.as_metric_labels();
    match error {
        FaultyBotError::User(error) => {
            increment_counter!("user_errors_total", &metric_labels);
            let error = error.to_string();
            send_message(error).await?;
        }
        _ => {
            increment_counter!("errors_total", &metric_labels);
            // Any other error means something went wrong while executing the command
            // Apologize to user and log
            tracing::error!("An error occured in a command: {}", error);

            send_message("Oops! Something went wrong! :(".to_string()).await?;
        }
    }

    Ok(())
}

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
                    .handle_message(ctx.clone(), framework, new_message.clone())
                    .await;
                if let Err(err) = result {
                    handle_error(err, new_message.into(), |msg| async move {
                        new_message.reply(ctx, msg).await?;
                        Ok(())
                    })
                    .await?;
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

        tracing::trace!("Received message: {:?}", new_message);

        // Only reply to DMs and direct mentions
        if new_message.guild_id.is_some() && !new_message.mentions_user_id(framework.bot_id) {
            return Ok(());
        }

        let channel_id = new_message
            .channel(&ctx)
            .await?
            .guild()
            .and_then(|c| c.thread_metadata.map(|_| c.parent_id))
            .flatten()
            .unwrap_or(new_message.channel_id);

        let persona = framework.user_data
            .persona_manager
            .get_active_persona(channel_id, new_message.guild_id)
            .await?;

        // validate access
        framework
            .user_data
            .permissions_manager
            .enforce(
                new_message.author.id,
                channel_id,
                new_message.guild_id,
                Permission::Chat(Some(persona.name())),
            )
            .await?;

        let cd_ctx = poise::CooldownContext {
            user_id: new_message.author.id,
            guild_id: new_message.guild_id,
            channel_id,
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
                return Err(UserError::cooldown_hit(time_remaining).into());
            }
        }

        let start = Instant::now();

        let author = new_message
            .author_nick(&ctx)
            .await
            .unwrap_or_else(|| new_message.author.name.clone());

        let metric_labels = AuditInfo::from(&new_message).as_metric_labels();
        increment_counter!("gpt_requests_total", &metric_labels);

        debug!(
            "Received message from {}: `{}`",
            author, &new_message.content
        );

        let result = Self::reply_with_gpt_completion(&ctx, persona, &new_message).await;

        if let Err(err) = result {
            increment_counter!("gpt_errors_total", &metric_labels);
            error!("Failed to send reply: {}", err);
        } else {
            increment_counter!("gpt_responses_total", &metric_labels);
        }

        let delay =
            serenity::Timestamp::now().fixed_offset() - new_message.timestamp.fixed_offset();
        let delay = delay.to_std().unwrap(); // duration can never be negative unless users send message in the future
        let duration = start.elapsed();
        histogram!("gpt_response_seconds", duration.as_secs_f64(), &metric_labels);
        histogram!("gpt_response_delay_seconds", delay.as_secs_f64(), &metric_labels);

        {
            self.cooldowns.write().await.start_cooldown(cd_ctx);
        }

        Ok(())
    }

    async fn reply_with_gpt_completion(
        ctx: &serenity::Context,
        persona: crate::gpt::Persona,
        message: &serenity::Message,
    ) -> Result<serenity::Message, Error> {
        let chat_completion = {
            let _typing = serenity::Typing::start(ctx.http.clone(), message.channel_id.0);

            let mut chat = Chat::from(ctx, persona, message).await?;

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
