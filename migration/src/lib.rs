mod m20230710_001739_create_permissions;
mod m20230722_210640_create_settings;
mod m20230805_205921_remove_guild_from_role;
mod m20230806_020929_create_personas;
mod m20230808_030829_seed_default_personas;
mod m20230830_031030_gpt_4;
mod m20240705_062830_gpt_4o;

pub use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20230710_001739_create_permissions::Migration),
            Box::new(m20230722_210640_create_settings::Migration),
            Box::new(m20230805_205921_remove_guild_from_role::Migration),
            Box::new(m20230806_020929_create_personas::Migration),
            Box::new(m20230808_030829_seed_default_personas::Migration),
            Box::new(m20230830_031030_gpt_4::Migration),
            Box::new(m20240705_062830_gpt_4o::Migration),
        ]
    }
}
