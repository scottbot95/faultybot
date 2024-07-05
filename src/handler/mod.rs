use std::fmt::Write;
use crate::gpt::Chat;
use metrics::{histogram, increment_counter};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::error::{FaultyBotError, UserError};
use crate::permissions::Permission;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{CacheHttp, Context, Message};
use tokio::sync::RwLock;
use crate::util::{AuditInfo, say_ephemeral};

const COOLDOWN_KEY: &str = "chat.cooldown";
const MAX_MESSAGE_SIZE: usize = 1950;

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) -> Result<(), Error> {
    match error {
        poise::FrameworkError::Command { ctx, error, .. } => {
            handle_error(error, (&ctx).into(), |msg| async move {
                say_ephemeral(ctx, msg, true).await?;
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
    pub fn new() -> Self {
        Self {
            cooldowns: RwLock::new(poise::Cooldowns::new()),
        }
    }

    pub async fn handle_event<'a>(
        &self,
        ctx: poise::FrameworkContext<'a, Data, Error>, 
        event: &'a serenity::FullEvent,
    ) -> Result<(), Error> {
        match event {
            serenity::FullEvent::Ready { .. } => {
                info!("Connected to discord!");
                poise::builtins::register_globally(ctx.serenity_context.http(), &ctx.options.commands).await?;
            }
            serenity::FullEvent::Resume { .. } => {
                info!("Connection resumed");
            }
            serenity::FullEvent::Message { new_message } => {
                let result = self
                    .handle_message(ctx, new_message.clone())
                    .await;
                if let Err(err) = result {
                    handle_error(err, new_message.into(), |msg| async move {
                        new_message.reply(ctx.serenity_context.http(), msg).await?;
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
        ctx: poise::FrameworkContext<'a, Data, Error>,
        new_message: serenity::Message,
    ) -> Result<(), Error> {
        // Ignore self messages
        if new_message.author.id == ctx.bot_id() {
            return Ok(());
        }

        tracing::trace!("Received message: {:?}", new_message);

        // Only reply to DMs and direct mentions
        if new_message.guild_id.is_some() && !new_message.mentions_user_id(ctx.bot_id()) {
            return Ok(());
        }

        let channel_id = new_message
            .channel(ctx.serenity_context)
            .await?
            .guild()
            .and_then(|c| c.thread_metadata.map(|_| c.parent_id))
            .flatten()
            .unwrap_or(new_message.channel_id);

        let persona = ctx.user_data()
            .persona_manager
            .get_active_persona(channel_id, new_message.guild_id)
            .await?;

        // validate access
        ctx.user_data()
            .permissions_manager
            .enforce(
                ctx,
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
            let config = self.get_config(cd_ctx.clone(), ctx.user_data()).await?;

            let time_remaining = self
                .cooldowns
                .read()
                .await
                .remaining_cooldown(cd_ctx.clone(), &config);
            if let Some(time_remaining) = time_remaining {
                return Err(UserError::cooldown_hit(time_remaining).into());
            }
        }

        let start = Instant::now();
        let msg_sent = new_message.timestamp;

        let author = new_message
            .author_nick(&ctx.serenity_context)
            .await
            .unwrap_or_else(|| new_message.author.name.clone().into_string());

        let metric_labels = AuditInfo::from(&new_message).as_metric_labels();
        increment_counter!("gpt_requests_total", &metric_labels);

        debug!(
            "Received message from {}: `{}`",
            author, &new_message.content
        );

        let result = Self::reply_with_gpt_completion(ctx.serenity_context, persona, new_message).await;

        if let Err(err) = result {
            increment_counter!("gpt_errors_total", &metric_labels);
            error!("Failed to send reply: {}", err);
        } else {
            increment_counter!("gpt_responses_total", &metric_labels);
        }

        let delay =
            serenity::Timestamp::now().fixed_offset() - msg_sent.fixed_offset();
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
        message: serenity::Message,
    ) -> Result<serenity::Message, Error> {
        let _typing = serenity::Typing::start(ctx.http.clone(), message.channel_id);

        let chat = Chat::from(ctx, persona, &message).await?;
        let stream = chat.stream_completion().await?;

        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(Self::produce_message_chunks(stream, tx));

        let mut last_msg: serenity::Message = message;
        while let Some(content) = rx.recv().await {
            last_msg = Self::send_reply(ctx, &last_msg, content).await?;
        }

        // Ok(last_msg.expect("ChatGPT API didn't return any response"))
        Ok(last_msg)
    }

    async fn produce_message_chunks(
        mut stream: impl tokio_stream::Stream<Item=openai::chat::ChatCompletionMessageDelta> + Unpin,
        tx: tokio::sync::mpsc::Sender<String>
    ) -> Result<(), Error> {
        use tokio_stream::StreamExt as _;

        let mut buffer = String::with_capacity(MAX_MESSAGE_SIZE);

        while let Some(delta) = stream.next().await {
            match delta.role {
                None => (),
                Some(openai::chat::ChatCompletionMessageRole::Assistant) => (),
                _ => continue // ignore everything not for the assistant
            }

            if let Some(content) = delta.content {
                buffer.write_str(content.as_str())?;

                while buffer.len() > MAX_MESSAGE_SIZE {
                    let chars_to_take = find_last_whitespace_before_index(&buffer, MAX_MESSAGE_SIZE).unwrap_or(MAX_MESSAGE_SIZE - 2 ) + 1;
                    tx.send(buffer[..chars_to_take].to_owned())
                        .await
                        .map_err(Error::boxed)?;
                    buffer = buffer[chars_to_take..].to_owned();
                }
            } else {
                tracing::warn!("Stream delta with no content detected: {:?}", delta);
            }
        }

        tx.send(buffer).await.map_err(Error::boxed)?;

        Ok(())
    }

    async fn send_reply(ctx: &Context, message: &Message, content: impl Into<String>) -> Result<Message, Error> {
        let content = content.into();
        tracing::debug!("Sending GPT response message: {}", content);

        let mut builder = serenity::CreateMessage::default()
            .content(content);
        if message.guild_id.is_some() {
            builder = builder
                .reference_message(message)
                // Disallow mentions
                .allowed_mentions(serenity::CreateAllowedMentions::default());
        }
        let result = message
            .channel_id
            .send_message(ctx.http(), builder)
            .await?;
        Ok(result)
    }

    async fn get_config(
        &self,
        ctx: poise::CooldownContext,
        user_data: Arc<Data>,
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

fn find_last_whitespace_before_index(input_str: &str, index: usize) -> Option<usize> {
    for (i, c) in input_str.char_indices().rev() {
        if i < index && c.is_whitespace() {
            return Some(i);
        }
    }
    None
}
