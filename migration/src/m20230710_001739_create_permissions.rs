use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Effect::Table)
                    .values(Effect::iter().skip(1))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Permission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Permission::Id)
                            .integer()
                            .auto_increment()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Permission::UserId).integer())
                    .col(ColumnDef::new(Permission::GuildId).integer())
                    .col(ColumnDef::new(Permission::RoleId).integer())
                    .col(ColumnDef::new(Permission::Action).string().not_null())
                    .col(
                        ColumnDef::new(Permission::Effect)
                            .enumeration(Effect::Table, Effect::iter().skip(1))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Permission::Until).timestamp())
                    .index(
                        Index::create()
                            .unique()
                            .name("PrincipleAction")
                            .col(Permission::GuildId)
                            .col(Permission::UserId)
                            .col(Permission::RoleId)
                            .col(Permission::Action)
                            .nulls_not_distinct(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Permission::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(Effect::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Permission {
    Table,
    Id,
    UserId,
    RoleId,
    GuildId,
    Action,
    Effect,
    Until,
}

#[derive(Iden, EnumIter)]
pub enum Effect {
    Table,
    #[iden = "Allow"]
    Allow,
    #[iden = "Deny"]
    Deny,
}
