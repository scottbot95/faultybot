use std::fmt::Write;
use crate::Error;
use migration::MigratorTrait;
use tracing::log::LevelFilter;
use crate::settings::GlobalSettings;

pub struct Database {
    connection: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(settings: &GlobalSettings) -> Result<Database, Error> {
        let url = match settings.database.url.clone() {
            Some(url) => url,
            None => {
                let mut url = "postgresql://".to_string();
                write!(&mut url, "/{}", settings.database.name.clone().expect("Must provide database.name if database.url is not set"))?;
                write!(&mut url, "?host={}", settings.database.host.as_deref().unwrap_or("localhost"))?;
                if let Some(port) = settings.database.port {
                    write!(&mut url, "&port={}", port)?;
                }
                if let Some(user) = settings.database.user.as_deref() {
                    write!(&mut url, "&user={}", user)?;
                }
                if let Some(password) = settings.database.password.as_deref() {
                    write!(&mut url, "&password={}", password)?;
                }
                for (key, value) in &settings.database.params {
                    write!(&mut url, "&{}={}", key, value)?;
                }
                url
            }
        };

        let mut opts = sea_orm::ConnectOptions::new(url);
        opts.sqlx_logging_level(LevelFilter::Trace);

        let connection = sea_orm::Database::connect(opts).await?;

        Ok(Database { connection })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        migration::Migrator::up(&self.connection, None).await?;
        Ok(())
    }
}
