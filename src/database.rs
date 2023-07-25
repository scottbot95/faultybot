use crate::Error;
use migration::MigratorTrait;
use std::fmt::Write;
use tracing::log::LevelFilter;

#[derive(Debug, Clone)]
pub struct Database {
    connection: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(settings: &crate::settings::config::Database) -> Result<Database, Error> {
        let url = match settings.url.clone() {
            Some(url) => url,
            None => {
                let mut url = "postgresql://".to_string();
                write!(
                    &mut url,
                    "/{}",
                    settings
                        .name
                        .clone()
                        .expect("Must provide database.name if database.url is not set")
                )?;
                write!(
                    &mut url,
                    "?host={}",
                    settings.host.as_deref().unwrap_or("localhost")
                )?;
                if let Some(port) = settings.port {
                    write!(&mut url, "&port={}", port)?;
                }
                if let Some(user) = &settings.user {
                    write!(&mut url, "&user={}", user)?;
                }
                if let Some(password) = &settings.password {
                    write!(&mut url, "&password={}", password)?;
                }
                if let Some(params) = &settings.params {
                    for (key, value) in params {
                        write!(&mut url, "&{}={}", key, value)?;
                    }
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

    pub(crate) fn connection(&self) -> &sea_orm::DatabaseConnection {
        &self.connection
    }
}
