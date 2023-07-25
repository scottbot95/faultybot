use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GuildSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GuildSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GuildSettings::GuildId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GuildSettings::Key).string().not_null())
                    .col(ColumnDef::new(GuildSettings::Value).json().not_null())
                    .index(
                        Index::create()
                            .name("GuildKey")
                            .unique()
                            .col(GuildSettings::GuildId)
                            .col(GuildSettings::Key),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ChannelSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChannelSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ChannelSettings::ChannelId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChannelSettings::Key).string().not_null())
                    .col(ColumnDef::new(ChannelSettings::Value).json().not_null())
                    .index(
                        Index::create()
                            .name("ChannelKey")
                            .unique()
                            .col(ChannelSettings::ChannelId)
                            .col(ChannelSettings::Key),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GuildUserSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GuildUserSettings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GuildUserSettings::GuildId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GuildUserSettings::UserId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GuildUserSettings::Key).string().not_null())
                    .col(ColumnDef::new(GuildUserSettings::Value).json().not_null())
                    .index(
                        Index::create()
                            .name("GuildUserKey")
                            .unique()
                            .col(GuildUserSettings::GuildId)
                            .col(GuildUserSettings::UserId)
                            .col(GuildUserSettings::Key)
                            .nulls_not_distinct(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GuildSettings::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ChannelSettings::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GuildUserSettings::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum ChannelSettings {
    Table,
    Id,
    ChannelId,
    Key,
    Value,
}

#[derive(Iden)]
enum GuildSettings {
    Table,
    Id,
    GuildId,
    Key,
    Value,
}

#[derive(Iden)]
enum GuildUserSettings {
    Table,
    Id,
    GuildId,
    UserId,
    Key,
    Value,
}
