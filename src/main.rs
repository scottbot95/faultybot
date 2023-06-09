mod commands;
mod database;
mod error;
mod gpt;
mod handler;
mod metrics;
mod settings;

use dotenvy::dotenv;
use openai::set_key;

use crate::commands::help;
use crate::metrics::{init_metrics, periodic_metrics};
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info};

use crate::settings::GlobalSettings;
use poise::{serenity_prelude as serenity, CooldownConfig};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    handler: handler::Handler,
    #[allow(dead_code)]
    db: database::Database,
}

#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long = "configFile")]
    cfg_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    dotenv().ok(); // ignore errors
    let settings = GlobalSettings::new(args.cfg_file).expect("Failed to load settings");

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_ansi(settings.ansi.colors)
        .init();

    init_metrics(&settings);

    set_key(settings.openai.key);

    info!("Connecting to database...");
    let db = database::Database::connect(settings.database.url)
        .await
        .expect("Failed to connect to database");
    db.migrate()
        .await
        .expect("Failed to update db to latest schema");
    info!("Database is connected and up-to-date");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![help()],
            event_handler: |ctx, event, framework, data: &Data| {
                Box::pin(async move {
                    data.handler
                        .handle_event(ctx, event, framework, data)
                        .await?;

                    Ok(())
                })
            },
            prefix_options: poise::PrefixFrameworkOptions {
                mention_as_prefix: false, // Disable mentions since we handle those directly
                ..Default::default()
            },
            ..Default::default()
        })
        .token(settings.discord.token)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
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

    periodic_metrics(
        framework.client().cache_and_http.cache.clone(),
        Duration::from_secs(60),
    );

    if let Err(err) = framework.start().await {
        error!("Poise framework error: {:?}", err);
    }
}
