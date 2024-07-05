use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{Context, GuildId, Message};
use tracing::debug;
use async_recursion::async_recursion;
use crate::Error;
use crate::error::FaultyBotError;
use crate::gpt::persona::Persona;

pub struct Chat {
    model: String,
    messages: Vec<ChatCompletionMessage>,
}

impl Chat {
    pub async fn from(ctx: &Context, persona: Persona, message: &Message) -> Result<Self, crate::Error> {
        let bot_name = bot_name(ctx, message.guild_id).await;
        let system_prompt = persona.prompt(bot_name.as_str());

        let mut instance = Self {
            model: persona.model(),
            messages: vec![ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(system_prompt),
                function_call: None,
                name: None,
            }],
        };

        let is_thread = message
            .channel(&ctx)
            .await?
            .guild()
            .map(|c| c.thread_metadata.is_some())
            .unwrap_or(false);

        if is_thread {
            instance.add_channel_messages(ctx, message).await?;
        } else {
            instance.add_message_chain(ctx, message).await;
        }

        debug!(
            "Starting chat as {}. With history: {:?}",
            persona.name(), instance.messages
        );

        Ok(instance)
    }

    pub async fn completion(&mut self) -> Result<ChatCompletionMessage, Error> {
        let completion = ChatCompletion::builder(&self.model, self.messages.clone())
            .create()
            .await?;

        let choice = completion.choices.first().unwrap().message.clone();

        self.messages.push(choice.clone());

        Ok(choice)
    }

    /// Returns a stream of completions for this [Chat].
    ///
    /// Does NOT update internal state and so take ownership of [Chat] to prevent accidental misuse
    pub async fn stream_completion(self) -> Result<impl tokio_stream::Stream<Item=openai::chat::ChatCompletionMessageDelta>, Error> {
        use tokio_stream::StreamExt as _;

        let rx = ChatCompletion::builder(&self.model, self.messages.clone())
            .create_stream()
            .await
            .map_err( FaultyBotError::boxed)?;

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
            .filter(|c| c.choices.first().unwrap().finish_reason.is_none())
            .map(|c| c.choices.first().unwrap().delta.clone());

        Ok(stream)
    }

    #[async_recursion]
    async fn add_message_chain(&mut self, ctx: &Context, message: &Message) {
        if let Some(message_reference) = &message.message_reference {
            let referenced = ctx
                .http
                .get_message(
                    message_reference.channel_id,
                    message_reference.message_id.unwrap(),
                )
                .await;
            if let Ok(referenced) = referenced {
                self.add_message_chain(ctx, &referenced).await;
            }
        }

        let bot_id = ctx.cache.current_user().id;

        let (role, name) = if message.author.id == bot_id {
            (ChatCompletionMessageRole::Assistant, None)
        } else {
            let author_nick = message
                .author_nick(ctx)
                .await
                .unwrap_or_else(|| message.author.name.clone().into_string());

            (ChatCompletionMessageRole::User, Some(author_nick))
        };

        let chat_message = ChatCompletionMessage {
            role,
            name,
            content: Some(message.content.clone().into_string()),
            function_call: None,
        };

        self.messages.push(chat_message);
    }

    async fn add_channel_messages(
        &mut self,
        ctx: &Context,
        message: &Message,
    ) -> Result<(), Error> {
        // TODO use .messages_iter with a Stream instead
        let messages_fut = message
            .channel_id
            .messages(ctx, serenity::GetMessages::default().limit(25).before(message.id))
            .await?
            .into_iter()
            .rev()
            .map(|m| async { m.into_chat_message(ctx).await });

        let mut messages = futures::future::join_all(messages_fut).await;
        self.messages.append(&mut messages);

        Ok(())
    }
}

async fn bot_name(ctx: &Context, guild_id: Option<GuildId>) -> String {
    let user = ctx.cache.current_user().clone();

    match guild_id {
        Some(guild_id) =>
            user.nick_in(ctx, guild_id)
                .await
                .unwrap_or_else(|| user.name.clone().into_string()),
        None => user.name.clone().into_string(),
    }
}

#[poise::async_trait]
trait IntoChatCompletionMessage {
    async fn into_chat_message(self, ctx: &Context) -> ChatCompletionMessage;
}

#[poise::async_trait]
impl IntoChatCompletionMessage for Message {
    async fn into_chat_message(self, ctx: &Context) -> ChatCompletionMessage {
        let (role, name) = if self.author.id == ctx.cache.current_user().id {
            (ChatCompletionMessageRole::Assistant, None)
        } else {
            let author_nick = self
                .author_nick(ctx)
                .await
                .unwrap_or_else(|| self.author.name.clone().into_string());

            (ChatCompletionMessageRole::User, Some(author_nick))
        };

        ChatCompletionMessage {
            role,
            name,
            content: Some(self.content.clone().into_string()),
            function_call: None,
        }
    }
}
