mod commands;
mod database;
mod error;
mod gpt;
mod handler;
mod metrics;
mod settings;

mod permissions;
#[cfg(test)]
mod test_util;
mod util;

use dotenvy::dotenv;

use crate::metrics::{init_metrics, periodic_metrics};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use octocrab::Octocrab;
use tracing::{error, info};

use crate::permissions::PermissionsManager;
use crate::settings::config::FaultybotConfig;
use poise::serenity_prelude as serenity;
use settings::manager::SettingsManager;
use tracing_subscriber::EnvFilter;

use database::Database;
use crate::gpt::PersonaManager;

type Error = error::FaultyBotError;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    handler: handler::Handler,
    settings_manager: SettingsManager,
    permissions_manager: PermissionsManager,
    config: FaultybotConfig,
    octocrab: Option<Octocrab>,
    persona_manager: PersonaManager,
}

#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long = "configFile")]
    cfg_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    dotenv().ok(); // ignore errors
    let config = settings::config::build_config(args.cfg_file).expect("Failed to load config");

    let settings: FaultybotConfig = config
        .clone()
        .try_deserialize()
        .expect("Failed to load settings");

    let config = Arc::new(config);

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .with_ansi(settings.ansi.colors)
        .init();

    tracing::debug!("{:?}", settings);

    init_metrics(&settings);

    openai::set_key(settings.openai.key.clone());

    let octocrab = settings.github.as_ref().map(|gh| Octocrab::builder()
        .personal_token(gh.token.clone())
        .build()
        .unwrap());

    info!("Connecting to database...");
    let db = database::Database::connect(&settings.database)
        .await
        .expect("Failed to connect to database");
    db.migrate()
        .await
        .expect("Failed to update db to latest schema");
    info!("Database is connected and up-to-date");

    let options = poise::FrameworkOptions {
        commands: commands::commands_vec(&settings),
        event_handler: |ctx, event| {
            Box::pin(async move {
                ctx.user_data().handler
                    .handle_event(ctx, event)
                    .await?;

                Ok(())
            })
        },
        pre_command: |ctx| Box::pin(async move {
            crate::metrics::record_command_start(ctx).await
        }),
        post_command: |ctx| Box::pin(async move {
            crate::metrics::record_command_completion(ctx).await
        }),
        on_error: |error| {
            Box::pin(async move {
                if let Err(e) = handler::on_error(error).await {
                    tracing::error!("Error while handling error: {}", e);
                }
            })
        },
        prefix_options: poise::PrefixFrameworkOptions {
            mention_as_prefix: false, // Disable mentions since we handle those directly
            ..Default::default()
        },
        ..Default::default()
    };

    let mut client = serenity::Client::builder(
        &settings.discord.token,
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
    )
        .framework(poise::Framework::new(options))
        .data(Arc::new(Data {
            config: settings,
            handler: handler::Handler::new(),
            settings_manager: SettingsManager::new(config, db.clone()),
            permissions_manager: PermissionsManager::new(db.clone()),
            octocrab,
            persona_manager: PersonaManager::new(db.clone()),
        }))
        .await
        .expect("Failed to create Poise Framework");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.shutdown_all().await;
    });

    periodic_metrics(
        client.cache.clone(),
        Duration::from_secs(60),
    );

    if let Err(err) = client.start().await {
        error!("Poise framework error: {:?}", err);
    }
}
