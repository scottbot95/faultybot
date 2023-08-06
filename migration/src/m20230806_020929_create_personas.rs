use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_type(
            Type::create()
                .as_enum(LLMModel::Table)
                .values(LLMModel::iter().skip(1))
                .to_owned(),
        ).await?;

        manager
            .create_table(
                Table::create()
                    .table(Persona::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Persona::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Persona::Name).string().not_null())
                    .col(ColumnDef::new(Persona::GuildId).big_unsigned().not_null())
                    .col(ColumnDef::new(Persona::Prompt).string().not_null())
                    .col(
                        ColumnDef::new(Persona::Model)
                            .enumeration(LLMModel::Table, LLMModel::iter().skip(1))
                            .not_null()
                    )
                    .index(Index::create()
                        .unique()
                        .name("NameGuildId")
                        .col(Persona::Name)
                        .col(Persona::GuildId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ActivePersona::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ActivePersona::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ActivePersona::GuildId).big_unsigned())
                    .col(ColumnDef::new(ActivePersona::ChannelId).big_unsigned())
                    .col(ColumnDef::new(ActivePersona::PersonaId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from_col(ActivePersona::PersonaId)
                            .to(Persona::Table, Persona::Id)
                            .on_delete(ForeignKeyAction::Cascade))
                    .index(Index::create()
                        .unique()
                        .name("ActivePersonaGuildChannel")
                        .col(ActivePersona::GuildId)
                        .col(ActivePersona::ChannelId)
                        .nulls_not_distinct())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ActivePersona::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Persona::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(LLMModel::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Persona {
    Table,
    Id,
    Name,
    GuildId,
    Model,
    Prompt,
}

#[derive(DeriveIden)]
enum ActivePersona {
    Table,
    Id,
    GuildId,
    ChannelId,
    PersonaId,
}

#[derive(DeriveIden, EnumIter)]
enum LLMModel {
    Table,
    #[sea_orm(iden = "gpt-3.5-turbo")]
    Gpt35Turbo,
}
