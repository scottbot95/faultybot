mod m20230710_001739_create_permissions;
mod m20230722_210640_create_settings;

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230710_001739_create_permissions::Migration),
            Box::new(m20230722_210640_create_settings::Migration),
        ]
    }
}
