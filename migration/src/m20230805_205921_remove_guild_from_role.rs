use sea_orm_migration::prelude::*;
use crate::m20230710_001739_create_permissions::RolePolicy;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_index(
            Index::drop()
                .table(RolePolicy::Table)
                .name("RoleGuild")
                .to_owned()
        ).await?;

        manager
            .alter_table(
                Table::alter()
                    .table(RolePolicy::Table)
                    .drop_column(RolePolicy::GuildId)
                    .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(RolePolicy::Table)
                    .add_column(
                        ColumnDef::new(RolePolicy::GuildId)
                            .big_unsigned()
                            .not_null()
                    )
                    .to_owned()
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RolePolicy::Table)
                    .name("RoleGuild")
                    .col(RolePolicy::GuildId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

