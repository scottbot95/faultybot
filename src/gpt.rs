use crate::BotInfoContainer;
use async_recursion::async_recursion;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use serenity::model::prelude::{GuildId, Message};
use serenity::prelude::Context;
use std::error::Error;
use tracing::debug;

pub struct Chat {
    model: String,
    messages: Vec<ChatCompletionMessage>,
}

impl Chat {
    pub async fn from(ctx: &Context, model: &str, message: &Message) -> Self {
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
                content: system_prompt,
                name: None,
            }],
        };

        instance.add_message_chain(ctx, message).await;

        debug!("Chat history: {:?}", instance.messages);

        instance
    }

    #[async_recursion]
    pub async fn add_message_chain(&mut self, ctx: &Context, message: &Message) {
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

        let bot_id = ctx.data.read().await.get::<BotInfoContainer>().unwrap().id;

        let (role, name) = if message.author.id == bot_id {
            (ChatCompletionMessageRole::Assistant, None)
        } else {
            let author_nick = message
                .author_nick(ctx)
                .await
                .unwrap_or_else(|| message.author.name.clone());

            (ChatCompletionMessageRole::User, Some(author_nick))
        };

        self.messages.push(ChatCompletionMessage {
            role,
            name,
            content: message.content.clone(),
        });
    }

    pub async fn completion(&mut self) -> Result<ChatCompletionMessage, Box<dyn Error>> {
        let completion = ChatCompletion::builder(&self.model, self.messages.clone())
            .create()
            .await??;

        let choice = completion.choices.first().unwrap().message.clone();

        self.messages.push(choice.clone());

        Ok(choice)
    }
}

async fn bot_name(ctx: &Context, guild_id: Option<GuildId>) -> String {
    let user = ctx
        .data
        .read()
        .await
        .get::<BotInfoContainer>()
        .unwrap()
        .clone();

    match guild_id {
        Some(guild_id) => user.nick_in(ctx, guild_id).await.unwrap_or(user.name),
        None => user.name,
    }
}
