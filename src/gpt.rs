use async_recursion::async_recursion;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use tracing::debug;

use crate::Error;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{Context, GuildId, Message};

pub struct Chat {
    model: String,
    messages: Vec<ChatCompletionMessage>,
}

impl Chat {
    pub async fn from(ctx: &Context, model: &str, message: &Message) -> Result<Self, crate::Error> {
        let bot_name = bot_name(ctx, message.guild_id).await;
        let system_prompt = format!(
            r#"You are {}, a helpful assistant build into a Discord bot.
            You are helpful, but your responses are always sassy and sometimes rude."#,
            bot_name
        )
        .trim()
        .to_string();

        let mut instance = Self {
            model: model.to_string(),
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

        debug!("Chat history: {:?}", instance.messages);

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

    #[async_recursion]
    async fn add_message_chain(&mut self, ctx: &Context, message: &Message) {
        if let Some(message_reference) = &message.message_reference {
            let referenced = ctx
                .http
                .get_message(
                    message_reference.channel_id.0,
                    message_reference.message_id.unwrap().0,
                )
                .await;
            if let Ok(referenced) = referenced {
                self.add_message_chain(ctx, &referenced).await;
            }
        }

        let bot_id = ctx.cache.current_user_id();

        let (role, name) = if message.author.id == bot_id {
            (ChatCompletionMessageRole::Assistant, None)
        } else {
            let author_nick = message
                .author_nick(ctx)
                .await
                .unwrap_or_else(|| message.author.name.clone());

            (ChatCompletionMessageRole::User, Some(author_nick))
        };

        let chat_message = ChatCompletionMessage {
            role,
            name,
            content: Some(message.content.clone()),
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
            .messages(ctx, |b| b.limit(25).before(message.id))
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
    let user: serenity::User = ctx.cache.current_user().into();

    match guild_id {
        Some(guild_id) => user.nick_in(ctx, guild_id).await.unwrap_or(user.name),
        None => user.name,
    }
}

#[poise::async_trait]
trait IntoChatCompletionMessage {
    async fn into_chat_message(self, ctx: &Context) -> ChatCompletionMessage;
}

#[poise::async_trait]
impl IntoChatCompletionMessage for Message {
    async fn into_chat_message(self, ctx: &Context) -> ChatCompletionMessage {
        let (role, name) = if self.author.id == ctx.cache.current_user_id() {
            (ChatCompletionMessageRole::Assistant, None)
        } else {
            let author_nick = self
                .author_nick(ctx)
                .await
                .unwrap_or_else(|| self.author.name.clone());

            (ChatCompletionMessageRole::User, Some(author_nick))
        };

        ChatCompletionMessage {
            role,
            name,
            content: Some(self.content.clone()),
            function_call: None,
        }
    }
}
