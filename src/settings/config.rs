use config::{Environment, File, Map, Source, Value};
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroU16;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Ansi {
    pub(crate) colors: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Database {
    pub(crate) url: Option<String>,
    pub(crate) host: Option<String>,
    pub(crate) port: Option<NonZeroU16>,
    pub(crate) name: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) params: Option<HashMap<String, String>>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Discord {
    pub(crate) token: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct GitHub {
    pub(crate) token: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) confirmation_channel: poise::serenity_prelude::ChannelId,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct OpenAI {
    pub(crate) key: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Prometheus {
    pub(crate) listen: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct Statsd {
    pub(crate) host: String,
    pub(crate) port: u16,
}

#[derive(Debug, Default, Deserialize)]
pub struct FaultybotConfig {
    pub(crate) ansi: Ansi,
    pub(crate) database: Database,
    pub(crate) discord: Discord,
    pub(crate) github: Option<GitHub>,
    pub(crate) openai: OpenAI,
    pub(crate) prometheus: Option<Prometheus>,
    pub(crate) statsd: Option<Statsd>,
}

#[derive(Debug)]
struct DefaultSettings;

impl Source for DefaultSettings {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(Self)
    }

    fn collect(&self) -> Result<Map<String, Value>, ::config::ConfigError> {
        let mut defaults = HashMap::new();
        defaults.insert("ansi.colors".to_string(), true.into());

        Ok(defaults)
    }
}

pub fn build_config(
    config_file: Option<PathBuf>,
) -> Result<::config::Config, ::config::ConfigError> {
    ::config::Config::builder()
        .add_source(DefaultSettings)
        .add_source(
            config_file
                .map(File::from)
                .unwrap_or_else(|| File::with_name("config/faultybot"))
                .required(false),
        )
        .add_source(Environment::default().separator("__"))
        .build()
}
