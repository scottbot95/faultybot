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

        // GuildPolicy table
        manager
            .create_table(
                Table::create()
                    .table(GuildPolicy::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GuildPolicy::Id)
                            .integer()
                            .auto_increment()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GuildPolicy::GuildId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GuildPolicy::Action).string().not_null())
                    .col(
                        ColumnDef::new(GuildPolicy::Effect)
                            .enumeration(Effect::Table, Effect::iter().skip(1))
                            .not_null(),
                    )
                    .col(ColumnDef::new(GuildPolicy::Until).timestamp_with_time_zone())
                    .index(
                        Index::create()
                            .unique()
                            .name("GuildAction")
                            .col(GuildPolicy::GuildId)
                            .col(GuildPolicy::Action),
                    )
                    .to_owned(),
            )
            .await?;

        // ChannelPolicy table
        manager
            .create_table(
                Table::create()
                    .table(ChannelPolicy::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChannelPolicy::Id)
                            .integer()
                            .auto_increment()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChannelPolicy::ChannelId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChannelPolicy::Action).string().not_null())
                    .col(
                        ColumnDef::new(ChannelPolicy::Effect)
                            .enumeration(Effect::Table, Effect::iter().skip(1))
                            .not_null(),
                    )
                    .col(ColumnDef::new(ChannelPolicy::Until).timestamp_with_time_zone())
                    .index(
                        Index::create()
                            .unique()
                            .name("ChannelAction")
                            .col(ChannelPolicy::ChannelId)
                            .col(ChannelPolicy::Action),
                    )
                    .to_owned(),
            )
            .await?;

        // RolePolicy table
        manager
            .create_table(
                Table::create()
                    .table(RolePolicy::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RolePolicy::Id)
                            .integer()
                            .auto_increment()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RolePolicy::RoleId).big_unsigned().not_null())
                    .col(
                        ColumnDef::new(RolePolicy::GuildId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RolePolicy::Action).string().not_null())
                    .col(
                        ColumnDef::new(RolePolicy::Effect)
                            .enumeration(Effect::Table, Effect::iter().skip(1))
                            .not_null(),
                    )
                    .col(ColumnDef::new(RolePolicy::Until).timestamp_with_time_zone())
                    .index(
                        Index::create()
                            .unique()
                            .name("RoleAction")
                            .col(RolePolicy::RoleId)
                            .col(RolePolicy::Action),
                    )
                    .to_owned(),
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

        // MemberPolicy table
        manager
            .create_table(
                Table::create()
                    .table(MemberPolicy::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MemberPolicy::Id)
                            .integer()
                            .auto_increment()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MemberPolicy::GuildId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MemberPolicy::UserId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MemberPolicy::Action).string().not_null())
                    .col(
                        ColumnDef::new(MemberPolicy::Effect)
                            .enumeration(Effect::Table, Effect::iter().skip(1))
                            .not_null(),
                    )
                    .col(ColumnDef::new(MemberPolicy::Until).timestamp_with_time_zone())
                    .index(
                        Index::create()
                            .unique()
                            .name("MemberAction")
                            .col(MemberPolicy::GuildId)
                            .col(MemberPolicy::UserId)
                            .col(MemberPolicy::Action),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GuildPolicy::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ChannelPolicy::Table).to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("RoleGuild")
                    .table(RolePolicy::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(RolePolicy::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(MemberPolicy::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(Effect::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GuildPolicy {
    Table,
    Id,
    GuildId,
    Action,
    Effect,
    Until,
}

#[derive(DeriveIden)]
enum ChannelPolicy {
    Table,
    Id,
    ChannelId,
    Action,
    Effect,
    Until,
}

#[derive(DeriveIden)]
enum RolePolicy {
    Table,
    Id,
    RoleId,
    GuildId,
    // Store guild for easy lookup
    Action,
    Effect,
    Until,
}

#[derive(DeriveIden)]
enum MemberPolicy {
    Table,
    Id,
    GuildId,
    UserId,
    Action,
    Effect,
    Until,
}

#[derive(DeriveIden, EnumIter)]
pub enum Effect {
    Table,
    #[sea_orm(iden = "Allow")]
    Allow,
    #[sea_orm(iden = "Deny")]
    Deny,
}
