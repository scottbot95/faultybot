mod gpt;

use std::collections::HashSet;
use std::env;
use std::sync::{Arc};
use dotenvy::dotenv;
use openai::set_key;
use serenity::{async_trait, Client};
use serenity::client::bridge::gateway::ShardManager;
use serenity::client::Context;
use serenity::framework::StandardFramework;
use serenity::http::{Http, Typing};
use serenity::model::channel::Message;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::model::prelude::CurrentUser;
use serenity::prelude::{EventHandler, GatewayIntents, TypeMapKey};
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use crate::gpt::Chat;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer { 
    type Value = Arc<Mutex<ShardManager>>; 
}

pub struct BotInfoContainer;

impl TypeMapKey for BotInfoContainer {
    type Value = CurrentUser;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, message: Message) {
        debug!("Received {:?}", message);

        if message.guild_id.is_some() && !message.mentions_me(&ctx).await.expect("Failed to read mentions") {
            return
        }

        {
            // Ignore message sent by the bot
            let data = ctx.data.read().await;
            if message.author.id == data.get::<BotInfoContainer>().unwrap().id {
                return;
            }
        }

        let author = message.author_nick(&ctx)
            .await
            .unwrap_or(message.author.name.clone());
        debug!("Received message from {}: `{}`", author, &message.content);

        let chat_completion = {
            let _typing = Typing::start(ctx.http.clone(), message.channel_id.0);

            let mut chat = Chat::new("gpt-3.5-turbo", "FaultyGPT");

            let completion = chat.chat_completion(message.content.clone(), None).await;
            match completion {
                Ok(completion) => completion,
                Err(err) => {
                    error!("Failed to generate chat completion: {}", err);
                    return;
                }
            }
        };

        let result = message.channel_id.send_message(&ctx, |builder| {
            if message.guild_id.is_some() {
                builder.reference_message(&message);
            }
            builder.content(chat_completion.content)
        }).await;

        if let Err(err) = result {
            error!("Failed to send reply: {}", err);
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // ignore errors
    set_key(env::var("OPENAI_KEY").expect("OPENAI_KEY must be set"));

    // Initialize the logger
    tracing_subscriber::fmt::init();

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");

    let http = Http::new(&discord_token);

    // Fetch owner info
    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let owners = HashSet::from([info.owner.id]);
            owners
        },
        Err(err) => panic!("Could not access application info: {:?}", err)
    };

    let bot_id = match http.get_current_user().await {
        Ok(bot_id) => bot_id,
        Err(err) => panic!("Could not access the bot id: {:?}", err)
    };

    // create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&discord_token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // Initialize client context
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<BotInfoContainer>(bot_id);
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(err) = client.start().await {
        error!("Client error: {:?}", err);
    }
}
