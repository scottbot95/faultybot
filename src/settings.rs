use config::{Config, ConfigError, Environment, File, Map, Source, Value};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;

pub trait Settings {}

#[derive(Debug, Deserialize)]
pub(crate) struct Ansi {
    pub(crate) colors: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Database {
    pub(crate) url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Discord {
    pub(crate) token: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAI {
    pub(crate) key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Prometheus {
    pub(crate) listen: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Statsd {
    pub(crate) host: String,
    pub(crate) port: u16,
}

#[derive(Debug, Deserialize)]
pub struct GlobalSettings {
    pub(crate) ansi: Ansi,
    pub(crate) database: Database,
    pub(crate) discord: Discord,
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

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let mut defaults = HashMap::new();
        defaults.insert("ansi.colors".to_string(), true.into());

        Ok(defaults)
    }
}

impl GlobalSettings {
    pub fn new(config_file: Option<PathBuf>) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(DefaultSettings)
            .add_source(
                config_file
                    .map(File::from)
                    .unwrap_or_else(|| File::with_name("config/faultybot"))
                    .required(false),
            )
            .add_source(Environment::default().separator("_"))
            .build()?;

        config.try_deserialize()
    }
}

impl Settings for GlobalSettings {}
