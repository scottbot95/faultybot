mod commands;
mod gpt;
mod handler;
mod metrics;
mod error;
mod database;

use dotenvy::dotenv;
use openai::set_key;

use std::env;
use std::time::Duration;
use tracing::{error, info};
use crate::commands::help;
use crate::metrics::{init_metrics, periodic_metrics};

use poise::{CooldownConfig, serenity_prelude as serenity};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    handler: handler::Handler,
    db: database::Database,
}

#[tokio::main]
async fn main() {
    dotenv().ok(); // ignore errors
    set_key(env::var("OPENAI_KEY").expect("OPENAI_KEY must be set"));

    let ansi_colors = env::var("ANSI_COLORS").map(|var| var.to_lowercase());
    let with_ansi = if let Ok(var) = ansi_colors {
        var != "false"
    } else {
        true
    };

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_ansi(with_ansi)
        .init();

    init_metrics();

    info!("Connecting to database...");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = database::Database::connect(db_url).await.expect("Failed to connect to database");
    db.migrate().await.expect("Failed to update db to latest schema");
    info!("Database is connected and up-to-date");

    let discord_token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![help()],
            event_handler: |ctx, event, framework, data: &Data| Box::pin(async move {
                data.handler.handle_event(ctx, event, framework, data).await?;

                Ok(())
            }),
            prefix_options: poise::PrefixFrameworkOptions {
                mention_as_prefix: false, // Disable mentions since we handle those directly
                ..Default::default()
            },
            ..Default::default()
        })
        .token(discord_token)
        .intents(serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT)
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    db,
                    handler: handler::Handler::new(CooldownConfig {
                        user: Some(Duration::from_secs(10)),
                        guild: Some(Duration::from_secs(5)),
                        ..Default::default()
                    }),
                })
            })
        })
        .build()
        .await
        .expect("Failed to create Poise Framework");


    let shard_manager = framework.shard_manager().clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    periodic_metrics(framework.client().cache_and_http.cache.clone(), Duration::from_secs(60));

    if let Err(err) = framework.start().await {
        error!("Poise framework error: {:?}", err);
    }
}
