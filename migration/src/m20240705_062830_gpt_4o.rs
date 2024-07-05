use sea_orm_migration::prelude::*;
use sea_orm_migration::prelude::extension::postgres::TypeAlterStatement;
use crate::m20230806_020929_create_personas::LLMModel;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_type(
            TypeAlterStatement::new()
                .name(LLMModel::Table)
                .rename_value(Alias::new("gpt-4"), Alias::new("gpt-4o"))
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_type(
            TypeAlterStatement::new()
                .name(LLMModel::Table)
                .rename_value(Alias::new("gpt-4o"), Alias::new("gpt-4"))
        ).await
    }
}
