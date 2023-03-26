mod commands;
mod framework;
mod gpt;
mod handler;

use crate::framework::build_framework;
use crate::handler::Handler;
use dotenvy::dotenv;
use openai::set_key;
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::StandardFramework;
use serenity::model::prelude::User;
use serenity::prelude::{GatewayIntents, TypeMapKey};
use serenity::Client;

use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct BotInfoContainer;

impl TypeMapKey for BotInfoContainer {
    type Value = User;
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // ignore errors
    set_key(env::var("OPENAI_KEY").expect("OPENAI_KEY must be set"));

    // Initialize the logger
    tracing_subscriber::fmt::init();

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");

    // create the framework
    let framework = build_framework(&discord_token).await;

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = build_client(&discord_token, framework, intents)
        .await
        .expect("Error creating client");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(err) = client.start().await {
        error!("Client error: {:?}", err);
    }
}

async fn build_client(
    token: &str,
    framework: StandardFramework,
    intents: GatewayIntents,
) -> Result<serenity::Client, serenity::Error> {
    let client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await?;

    let bot_user = match client.cache_and_http.http.get_current_user().await {
        Ok(bot_user) => bot_user,
        Err(err) => panic!("Could not access the bot id: {:?}", err),
    };

    // Initialize client context
    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<BotInfoContainer>(bot_user.into());
    }

    Ok(client)
}
