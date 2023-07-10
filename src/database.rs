use tracing::log::LevelFilter;
use migration::MigratorTrait;
use crate::Error;

pub struct Database {
    connection: sea_orm::DatabaseConnection,
}

impl Database {
    pub async fn connect(url: String) -> Result<Database, Error> {
        let mut opts = sea_orm::ConnectOptions::new(url);
        opts.sqlx_logging_level(LevelFilter::Trace);

        let connection = sea_orm::Database::connect(opts).await?;

        Ok(Database {
            connection,
        })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        migration::Migrator::up(&self.connection, None).await?;
        Ok(())
    }
}

